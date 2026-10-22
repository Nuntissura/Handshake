use super::*;

pub(super) async fn dispatch_router<S>(mut service: S, request: axum::extract::Request) -> axum::response::Response
where S: axum::ServiceExt<axum::extract::Request, Response = axum::response::Response, Error = std::convert::Infallible> {
    std::future::poll_fn(|context| service.poll_ready(context)).await.unwrap();
    service.call(request).await.unwrap()
}

#[cfg(all(feature = "test-utils", feature = "duckdb-flight-recorder"))]
pub(super) struct RecorderProofAttachment {
    control: Arc<crate::flight_recorder::shutdown_proof::RecorderProofControl>,
    task: tokio::task::JoinHandle<()>,
}

#[cfg(all(feature = "test-utils", feature = "duckdb-flight-recorder"))]
impl Drop for RecorderProofAttachment {
    fn drop(&mut self) {
        if !self.task.is_finished() {
            self.control.disconnect("proof attachment closed");
            self.task.abort();
        }
    }
}

#[cfg(all(feature = "test-utils", feature = "duckdb-flight-recorder"))]
async fn proof_transport_wait<T>(
    control: &crate::flight_recorder::shutdown_proof::RecorderProofControl,
    shutdown_rx: tokio::sync::watch::Receiver<bool>,
    future: impl std::future::Future<Output = T>,
) -> Result<T, String> {
    tokio::select! {
        _ = wait_for_shutdown_request(shutdown_rx) => Err("normal shutdown interrupted proof transport".into()),
        result = async {
            match control.operation_deadline() {
                Some(deadline) => tokio::time::timeout_at(deadline, future).await
                    .map_err(|_| "proof transport deadline exceeded".to_string()),
                None => Ok(future.await),
            }
        } => result,
    }
}

#[cfg(all(feature = "test-utils", feature = "duckdb-flight-recorder"))]
pub(super) fn attach_recorder_shutdown_proof(
    control: Arc<crate::flight_recorder::shutdown_proof::RecorderProofControl>,
    shutdown_tx: tokio::sync::watch::Sender<bool>,
    shutdown_rx: tokio::sync::watch::Receiver<bool>,
) -> io::Result<RecorderProofAttachment> {
    use crate::flight_recorder::shutdown_proof::{
        read_command, write_receipt, ProofAction, ProofCommand, ProofObservation, ProofReceipt,
    };
    use tokio::sync::{mpsc, oneshot};

    let (command_tx, mut command_rx) = mpsc::channel::<ProofCommand>(1);
    let (receipt_tx, mut receipt_rx) =
        mpsc::channel::<(ProofReceipt, oneshot::Sender<Result<(), String>>)>(1);

    let reader_control = control.clone();
    let _reader = std::thread::Builder::new().name("mt188-proof-reader".into()).spawn(move || {
        let stdin = io::stdin();
        let mut reader = stdin.lock();
        loop {
            match read_command(&mut reader) {
                Ok(Some(command)) => {
                    if command_tx.try_send(command).is_err() {
                        reader_control.disconnect("proof command stream closed or backpressured");
                        return;
                    }
                }
                Ok(None) => {
                    reader_control.disconnect("proof input EOF");
                    return;
                }
                Err(reason) => {
                    reader_control.disconnect(&reason);
                    return;
                }
            }
        }
    }).map_err(|error| {
        control.disconnect("proof reader thread failed to start");
        error
    })?;

    let writer_control = control.clone();
    let _writer = std::thread::Builder::new().name("mt188-proof-writer".into()).spawn(move || {
        let stdout = io::stdout();
        let mut writer = stdout.lock();
        while let Some((receipt, acknowledge)) = receipt_rx.blocking_recv() {
            let result = write_receipt(&mut writer, &receipt);
            if let Err(reason) = &result {
                writer_control.disconnect(reason);
            }
            let failed = result.is_err();
            let _ = acknowledge.send(result);
            if failed {
                return;
            }
        }
    }).map_err(|error| {
        control.disconnect("proof writer thread failed to start");
        error
    })?;

    let dispatcher_control = control.clone();
    let task = tokio::spawn(async move {
        let control = dispatcher_control;
        loop {
            let command = match proof_transport_wait(
                &control, shutdown_rx.clone(), command_rx.recv(),
            ).await {
                Ok(Some(command)) => command,
                Ok(None) => {
                    control.disconnect("proof input closed");
                    return;
                }
                Err(reason) => {
                    control.disconnect(&reason);
                    return;
                }
            };
            let requested_shutdown = matches!(&command.command, ProofAction::Shutdown);
            let receipt = match proof_transport_wait(
                &control, shutdown_rx.clone(), control.execute(command),
            ).await {
                Ok(receipt) => receipt,
                Err(reason) => {
                    control.disconnect(&reason);
                    return;
                }
            };
            let shutdown = requested_shutdown
                && matches!(&receipt.receipt, ProofObservation::ShutdownRequested);
            let incomplete = matches!(&receipt.receipt, ProofObservation::Incomplete { .. });
            let (acknowledge, acknowledged) = oneshot::channel();
            if receipt_tx.try_send((receipt, acknowledge)).is_err() {
                control.disconnect("proof receipt stream closed or backpressured");
                return;
            }
            let delivery = async {
                acknowledged.await
                    .map_err(|_| "proof receipt acknowledgment closed".to_string())?
            };
            match proof_transport_wait(&control, shutdown_rx.clone(), delivery).await {
                Ok(Ok(())) => {}
                Ok(Err(reason)) | Err(reason) => {
                    control.disconnect(&reason);
                    return;
                }
            }
            if shutdown {
                if !control.operation_deadline()
                    .is_some_and(|deadline| tokio::time::Instant::now() < deadline)
                {
                    control.disconnect("proof shutdown deadline exceeded");
                    return;
                }
                if shutdown_tx.send(true).is_err() {
                    control.disconnect("normal shutdown channel closed");
                }
                return;
            }
            if incomplete {
                return;
            }
        }
    });
    Ok(RecorderProofAttachment { control, task })
}

pub(super) async fn wait_for_shutdown_request(mut shutdown_rx: tokio::sync::watch::Receiver<bool>) {
    if *shutdown_rx.borrow() {
        return;
    }
    while shutdown_rx.changed().await.is_ok() {
        if *shutdown_rx.borrow() {
            return;
        }
    }
}

