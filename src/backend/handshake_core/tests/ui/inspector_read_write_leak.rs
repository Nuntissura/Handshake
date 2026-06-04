use handshake_core::inspector_read::{EventLedgerRow, InspectorReadV1};

fn main() {
    let reader: Option<&dyn InspectorReadV1> = None;
    if let Some(reader) = reader {
        reader.append_event(EventLedgerRow::default());
    }
}
