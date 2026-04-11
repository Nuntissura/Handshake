import assert from "node:assert/strict";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import test from "node:test";

import {
  ageSecondsFromEvent,
  findLatestAgentMessageEvent,
  findLatestProgressEvent,
  inspectSessionOutputActivity,
  isProgressEvent,
  isProgressItemType,
  itemTypeOfEvent,
  parseSessionOutputEntries,
  summarizeActivityEvent,
} from "../scripts/session/session-output-activity-lib.mjs";

function writeJsonl(entries) {
  const filePath = path.join(
    os.tmpdir(),
    `session-output-activity-${process.pid}-${Date.now()}-${Math.random().toString(16).slice(2)}.jsonl`,
  );
  fs.writeFileSync(
    filePath,
    `${entries.map((entry) => JSON.stringify(entry)).join("\n")}\n`,
    "utf8",
  );
  return filePath;
}

test("progress detection treats ACP tool-like item types as activity", () => {
  assert.equal(isProgressItemType("command_execution"), true);
  assert.equal(isProgressItemType("file_change"), true);
  assert.equal(isProgressItemType("web_search"), true);
  assert.equal(isProgressItemType("todo_list"), true);
  assert.equal(isProgressItemType("agent_message"), false);

  assert.equal(isProgressEvent({
    type: "item.completed",
    item: { type: "file_change" },
  }), true);
  assert.equal(isProgressEvent({
    type: "item.completed",
    item: { type: "agent_message" },
  }), false);
});

test("latest progress event can be a file change or web search, not only a command", () => {
  const entries = [
    { timestamp: "2026-04-11T00:00:00.000Z", type: "item.completed", item: { type: "command_execution" } },
    { timestamp: "2026-04-11T00:00:30.000Z", type: "item.completed", item: { type: "agent_message", text: "Thinking." } },
    { timestamp: "2026-04-11T00:01:00.000Z", type: "item.completed", item: { type: "web_search" } },
    { timestamp: "2026-04-11T00:01:30.000Z", type: "item.completed", item: { type: "file_change" } },
  ];

  const latestProgress = findLatestProgressEvent(entries);
  assert.equal(itemTypeOfEvent(latestProgress), "file_change");

  const latestMessage = findLatestAgentMessageEvent(entries);
  assert.equal(itemTypeOfEvent(latestMessage), "agent_message");
});

test("parse and inspect session output activity returns latest progress summaries", () => {
  const filePath = writeJsonl([
    { timestamp: "2026-04-11T00:00:00.000Z", type: "item.started", item: { id: "1", type: "command_execution" } },
    { timestamp: "2026-04-11T00:00:05.000Z", type: "item.completed", item: { id: "2", type: "agent_message", text: "I will edit files now." } },
    { timestamp: "2026-04-11T00:00:08.000Z", type: "item.completed", item: { id: "3", type: "file_change" } },
  ]);

  const entries = parseSessionOutputEntries(filePath, { tailLines: 10 });
  assert.equal(entries.length, 3);

  const activity = inspectSessionOutputActivity(filePath, {
    tailLines: 10,
    nowMs: Date.parse("2026-04-11T00:00:10.000Z"),
  });
  assert.equal(activity.exists, true);
  assert.equal(itemTypeOfEvent(activity.latestProgressEvent), "file_change");
  assert.equal(itemTypeOfEvent(activity.latestAgentMessageEvent), "agent_message");
  assert.equal(ageSecondsFromEvent(activity.latestProgressEvent, Date.parse("2026-04-11T00:00:10.000Z")), 2);
  assert.equal(
    summarizeActivityEvent(activity.latestProgressEvent, { nowMs: Date.parse("2026-04-11T00:00:10.000Z") }),
    "item.completed:file_change@2s",
  );

  fs.unlinkSync(filePath);
});
