use handshake_core::model_manual::{model_manual, render_model_manual_markdown};

fn main() {
    print!("{}", render_model_manual_markdown(model_manual()));
}
