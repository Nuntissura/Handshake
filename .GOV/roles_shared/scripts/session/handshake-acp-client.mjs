import fs from "node:fs";
import net from "node:net";
import path from "node:path";
import { spawn, spawnSync } from "node:child_process";
import {
  SESSION_CONTROL_BROKER_AUTH_MODE,
  SESSION_CONTROL_BROKER_BUILD_ID,
  SESSION_CONTROL_BROKER_STATE_FILE,
  SESSION_CONTROL_BROKER_SHUTDOWN_GRACE_SECONDS,
  SESSION_CONTROL_RUN_STALE_GRACE_SECONDS,
  SESSION_CONTROL_RUN_TIMEOUT_SECONDS,
} from "./session-policy.mjs";
import { ensureBrokerAuthToken } from "./session-control-lib.mjs";

export const HANDSHAKE_ACP_AGENT_REL_PATH = ".GOV/tools/handshake-acp-bridge/agent.mjs";

function readJson(filePath, fallbackValue) {
  if (!fs.existsSync(filePath)) return fallbackValue;
  return JSON.parse(fs.readFileSync(filePath, "utf8"));
}

function brokerStatePath(repoRoot) {
  return path.resolve(repoRoot, SESSION_CONTROL_BROKER_STATE_FILE);
}

function isProcessAlive(pid) {
  const numeric = Number(pid || 0);
  if (!Number.isInteger(numeric) || numeric <= 0) return false;
  try {
    process.kill(numeric, 0);
    return true;
  } catch {
    return false;
  }
}

const PROCESS_EXIT_POLL_MS = 150;
const PROCESS_EXIT_TIMEOUT_MS = 5000;
const WIN_TASKKILL_TIMEOUT_MS = 5000;

export function killProcessTree(
  pid,
  {
    platform = process.platform,
    taskkill = spawnSync,
    killer = process.kill.bind(process),
    taskkillTimeoutMs = WIN_TASKKILL_TIMEOUT_MS,
  } = {},
) {
  const numeric = Number(pid || 0);
  if (!Number.isInteger(numeric) || numeric <= 0) {
    return { attempted: false, usedFallback: false };
  }
  try {
    if (platform === "win32") {
      const result = taskkill(
        "taskkill",
        ["/PID", String(numeric), "/T", "/F"],
        {
          stdio: "ignore",
          windowsHide: true,
          timeout: taskkillTimeoutMs,
        },
      );
      if (!result?.error && result?.status === 0) {
        return { attempted: true, usedFallback: false };
      }
      try {
        killer(numeric, "SIGTERM");
        return { attempted: true, usedFallback: true };
      } catch {
        return { attempted: true, usedFallback: true };
      }
    }
    killer(numeric, "SIGTERM");
    return { attempted: true, usedFallback: false };
  } catch {
    // Ignore stale or already-dead broker processes.
    return { attempted: true, usedFallback: false };
  }
}

export async function waitForProcessExit(
  pid,
  timeoutMs = PROCESS_EXIT_TIMEOUT_MS,
  { isAlive = isProcessAlive } = {},
) {
  const numeric = Number(pid || 0);
  if (!Number.isInteger(numeric) || numeric <= 0) return true;
  const startedAt = Date.now();
  while ((Date.now() - startedAt) < timeoutMs) {
    if (!isAlive(numeric)) return true;
    await new Promise((resolve) => setTimeout(resolve, PROCESS_EXIT_POLL_MS));
  }
  return !isAlive(numeric);
}

async function canConnect(state, timeoutMs = 750) {
  if (!state?.host || !state?.port) return false;
  return await new Promise((resolve) => {
    const socket = net.createConnection({ host: state.host, port: state.port });
    let settled = false;
    const finish = (value) => {
      if (settled) return;
      settled = true;
      socket.destroy();
      resolve(value);
    };
    socket.setTimeout(timeoutMs);
    socket.once("connect", () => finish(true));
    socket.once("timeout", () => finish(false));
    socket.once("error", () => finish(false));
  });
}

async function waitForBroker(repoRoot, expectedPid, timeoutMs = 10000) {
  const startedAt = Date.now();
  const filePath = brokerStatePath(repoRoot);
  while ((Date.now() - startedAt) < timeoutMs) {
    const state = readJson(filePath, null);
    if (
      state?.broker_pid === expectedPid
      && state?.broker_build_id === SESSION_CONTROL_BROKER_BUILD_ID
      && state?.broker_auth_mode === SESSION_CONTROL_BROKER_AUTH_MODE
      && await canConnect(state)
    ) {
      return state;
    }
    await new Promise((resolve) => setTimeout(resolve, 150));
  }
  throw new Error(`Handshake ACP broker did not become ready within ${timeoutMs}ms`);
}

async function spawnBroker(repoRoot) {
  ensureBrokerAuthToken(repoRoot);
  const agentPath = path.resolve(repoRoot, HANDSHAKE_ACP_AGENT_REL_PATH);
  const child = spawn(process.execPath, [agentPath], {
    cwd: repoRoot,
    detached: true,
    windowsHide: true,
    stdio: "ignore",
  });
  child.unref();
  return await waitForBroker(repoRoot, child.pid || 0);
}

function activeRunsAreStale(state) {
  const runs = Array.isArray(state?.active_runs) ? state.active_runs : [];
  if (runs.length === 0) return true;
  return runs.every((run) => {
    const timeoutAtMs = Date.parse(run.timeout_at || "");
    return !Number.isNaN(timeoutAtMs)
      && Date.now() > (timeoutAtMs + (SESSION_CONTROL_RUN_STALE_GRACE_SECONDS * 1000));
  });
}

async function callBrokerRpc({ broker, authToken, method, params = {}, timeoutMs = 10000 }) {
  return await new Promise((resolve, reject) => {
    const socket = net.createConnection({ host: broker.host, port: broker.port });
    socket.setEncoding("utf8");

    let settled = false;
    let initialized = false;
    let methodResponse = null;
    let buffer = "";
    const timer = setTimeout(() => finish(new Error(`Handshake ACP broker ${method} timed out after ${timeoutMs}ms`), true), timeoutMs);

    const finish = (value, isError = false) => {
      if (settled) return;
      settled = true;
      clearTimeout(timer);
      socket.destroy();
      if (isError) reject(value);
      else resolve(value);
    };

    const handleLine = (line) => {
      if (!line.trim()) return;
      let message;
      try {
        message = JSON.parse(line);
      } catch (error) {
        finish(new Error(`Handshake ACP broker emitted invalid JSON: ${error.message}`), true);
        return;
      }

      if (message.id === 1) {
        if (message.error) {
          finish(new Error(message.error.message || "Handshake ACP initialize failed"), true);
          return;
        }
        initialized = true;
        if ((message.result?.broker_build_id || "") !== SESSION_CONTROL_BROKER_BUILD_ID) {
          finish(new Error(`Handshake ACP broker build mismatch: expected ${SESSION_CONTROL_BROKER_BUILD_ID}, got ${message.result?.broker_build_id || "<missing>"}`), true);
          return;
        }
        sendJson(socket, {
          jsonrpc: "2.0",
          id: 2,
          method,
          params,
        });
        return;
      }

      if (message.id === 2) {
        if (message.error) {
          finish(new Error(message.error.message || `Handshake ACP ${method} failed`), true);
          return;
        }
        methodResponse = message.result || null;
        finish(methodResponse);
      }
    };

    socket.on("connect", () => {
      sendJson(socket, {
        jsonrpc: "2.0",
        id: 1,
        method: "initialize",
        params: {
          protocol_version: "1.0",
          auth_token: authToken,
          expected_broker_build_id: SESSION_CONTROL_BROKER_BUILD_ID,
          expected_auth_mode: SESSION_CONTROL_BROKER_AUTH_MODE,
          authority_role: "ORCHESTRATOR",
          authority_branch: "gov_kernel",
          client: {
            name: "handshake-governance",
            version: "1.0.0",
          },
        },
      });
    });

    socket.on("data", (chunk) => {
      buffer += chunk.toString("utf8");
      const lines = buffer.split(/\r?\n/);
      buffer = lines.pop() || "";
      for (const line of lines) handleLine(line);
    });

    socket.on("error", (error) => finish(error, true));
    socket.on("close", () => {
      if (buffer.trim()) handleLine(buffer.trim());
      if (!settled && !initialized) finish(new Error("Handshake ACP broker closed before initialize"), true);
      else if (!settled && !methodResponse) finish(new Error(`Handshake ACP broker closed before ${method} response`), true);
    });
  });
}

async function shutdownBrokerGracefully(state, authToken) {
  if (!state?.host || !state?.port) return false;
  try {
    await callBrokerRpc({
      broker: state,
      authToken,
      method: "broker/shutdown",
      params: {},
      timeoutMs: SESSION_CONTROL_BROKER_SHUTDOWN_GRACE_SECONDS * 1000,
    });
    return true;
  } catch {
    return false;
  }
}

export async function inspectHandshakeAcpBroker(repoRoot) {
  const state = readJson(brokerStatePath(repoRoot), null);
  const authToken = ensureBrokerAuthToken(repoRoot);
  const brokerIsAlive = Boolean(state?.broker_pid && isProcessAlive(state.broker_pid));
  const brokerIsReachable = brokerIsAlive && await canConnect(state);
  const buildMatches = state?.broker_build_id === SESSION_CONTROL_BROKER_BUILD_ID
    && state?.broker_auth_mode === SESSION_CONTROL_BROKER_AUTH_MODE;
  return {
    state,
    authToken,
    brokerIsAlive,
    brokerIsReachable,
    buildMatches,
  };
}

export async function shutdownHandshakeAcpBroker(repoRoot, { force = false } = {}) {
  const inspected = await inspectHandshakeAcpBroker(repoRoot);
  const { state, authToken, brokerIsAlive, brokerIsReachable, buildMatches } = inspected;
  if (!brokerIsAlive) {
    return {
      status: "not_running",
      broker: state || null,
    };
  }

  const activeRunCount = Array.isArray(state?.active_runs) ? state.active_runs.length : 0;
  if (!buildMatches) {
    if (activeRunCount > 0 && !force) {
      return {
        status: "build_mismatch_with_active_runs",
        broker: state,
      };
    }
    killProcessTree(state?.broker_pid || 0);
    const exited = await waitForProcessExit(state?.broker_pid || 0);
    return {
      status: exited ? "killed_mismatched_broker" : "kill_requested_but_broker_still_alive",
      broker: state,
    };
  }

  if (!brokerIsReachable) {
    if (activeRunCount > 0 && !force) {
      return {
        status: "unreachable_with_active_runs",
        broker: state,
      };
    }
    killProcessTree(state?.broker_pid || 0);
    const exited = await waitForProcessExit(state?.broker_pid || 0);
    return {
      status: exited ? "killed_unreachable_broker" : "kill_requested_but_broker_still_alive",
      broker: state,
    };
  }

  const result = await callBrokerRpc({
    broker: state,
    authToken,
    method: "broker/shutdown",
    params: { force: Boolean(force) },
    timeoutMs: SESSION_CONTROL_BROKER_SHUTDOWN_GRACE_SECONDS * 1000,
  });
  return {
    status: result?.status || "shutdown_requested",
    broker: state,
    result,
  };
}

async function ensureBroker(repoRoot) {
  const inspected = await inspectHandshakeAcpBroker(repoRoot);
  const { state, authToken, brokerIsAlive, brokerIsReachable, buildMatches } = inspected;

  if (brokerIsAlive && brokerIsReachable && buildMatches) {
    return { ...state, auth_token: authToken };
  }

  if (brokerIsAlive && brokerIsReachable && !buildMatches) {
    if (Array.isArray(state?.active_runs) && state.active_runs.length > 0) {
      throw new Error(`Handshake ACP broker build mismatch while active runs exist; drain or cancel governed runs before restart (current=${state.broker_build_id || "<missing>"}, expected=${SESSION_CONTROL_BROKER_BUILD_ID})`);
    }
    const graceful = await shutdownBrokerGracefully(state, authToken);
    let brokerStopped = graceful
      ? await waitForProcessExit(state?.broker_pid || 0)
      : false;
    if (!brokerStopped && isProcessAlive(state.broker_pid)) {
      killProcessTree(state.broker_pid);
      brokerStopped = await waitForProcessExit(state?.broker_pid || 0);
    }
    if (!brokerStopped) {
      throw new Error(`Handshake ACP stale broker could not be stopped before restart (pid=${state?.broker_pid || "<missing>"})`);
    }
    return { ...(await spawnBroker(repoRoot)), auth_token: authToken };
  }

  if (brokerIsAlive && !brokerIsReachable && Array.isArray(state?.active_runs) && state.active_runs.length > 0 && !activeRunsAreStale(state)) {
    throw new Error("Handshake ACP broker is unreachable while governed runs are still active; refusing forced restart");
  }

  if (brokerIsAlive) {
    killProcessTree(state.broker_pid);
    const brokerStopped = await waitForProcessExit(state?.broker_pid || 0);
    if (!brokerStopped) {
      throw new Error(`Handshake ACP broker could not be stopped before restart (pid=${state?.broker_pid || "<missing>"})`);
    }
  }
  return { ...(await spawnBroker(repoRoot)), auth_token: authToken };
}

function sendJson(socket, value) {
  socket.write(`${JSON.stringify(value)}\n`);
}

export async function callHandshakeAcpMethod({
  repoRoot,
  method,
  params = {},
  timeoutMs = (SESSION_CONTROL_RUN_TIMEOUT_SECONDS + SESSION_CONTROL_RUN_STALE_GRACE_SECONDS + 30) * 1000,
  onNotification = null,
}) {
  const broker = await ensureBroker(repoRoot);
  return await new Promise((resolve, reject) => {
    const socket = net.createConnection({ host: broker.host, port: broker.port });
    socket.setEncoding("utf8");

    let settled = false;
    let initialized = false;
    let methodSent = false;
    let methodResponse = null;
    let buffer = "";
    const notifications = [];
    const timer = timeoutMs > 0
      ? setTimeout(() => {
        finish(new Error(`Handshake ACP call timed out after ${timeoutMs}ms`), true);
      }, timeoutMs)
      : null;

    const finish = (value, isError = false) => {
      if (settled) return;
      settled = true;
      if (timer) clearTimeout(timer);
      socket.destroy();
      if (isError) reject(value);
      else resolve(value);
    };

    const handleLine = (line) => {
      if (!line.trim()) return;
      let message;
      try {
        message = JSON.parse(line);
      } catch (error) {
        finish(new Error(`Handshake ACP broker emitted invalid JSON: ${error.message}`), true);
        return;
      }

      if (message.id === 1) {
        if (message.error) {
          finish(new Error(message.error.message || "Handshake ACP initialize failed"), true);
          return;
        }
        if ((message.result?.broker_build_id || "") !== SESSION_CONTROL_BROKER_BUILD_ID) {
          finish(new Error(`Handshake ACP broker build mismatch: expected ${SESSION_CONTROL_BROKER_BUILD_ID}, got ${message.result?.broker_build_id || "<missing>"}`), true);
          return;
        }
        initialized = true;
        if (!methodSent) {
          sendJson(socket, {
            jsonrpc: "2.0",
            id: 2,
            method,
            params,
          });
          methodSent = true;
        }
        return;
      }

      if (message.id === 2) {
        if (message.error) {
          finish(new Error(message.error.message || `Handshake ACP ${method} failed`), true);
          return;
        }
        methodResponse = message.result || null;
        finish({
          result: methodResponse,
          notifications,
          broker,
        });
        return;
      }

      if (message.method) {
        notifications.push(message);
        if (typeof onNotification === "function") onNotification(message);
      }
    };

    socket.on("connect", () => {
      sendJson(socket, {
        jsonrpc: "2.0",
        id: 1,
        method: "initialize",
        params: {
          protocol_version: "1.0",
            client: {
              name: "handshake-governance",
              version: "1.0.0",
            },
            auth_token: broker.auth_token,
            expected_broker_build_id: SESSION_CONTROL_BROKER_BUILD_ID,
            expected_auth_mode: SESSION_CONTROL_BROKER_AUTH_MODE,
            authority_role: "ORCHESTRATOR",
            authority_branch: "gov_kernel",
          },
        });
      });

    socket.on("data", (chunk) => {
      buffer += chunk.toString("utf8");
      const lines = buffer.split(/\r?\n/);
      buffer = lines.pop() || "";
      for (const line of lines) handleLine(line);
    });

    socket.on("error", (error) => {
      finish(error, true);
    });

    socket.on("close", () => {
      if (buffer.trim()) handleLine(buffer.trim());
      if (!settled && !initialized) {
        finish(new Error("Handshake ACP broker closed before initialize"), true);
      } else if (!settled && !methodResponse) {
        finish(new Error(`Handshake ACP broker closed before ${method} response`), true);
      }
    });
  });
}
