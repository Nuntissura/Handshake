//! The window library's loop, and its callbacks.
//!
//! The library holds back every event that arrives while one of its callbacks runs. So none of them calls the host:
//! they translate, create windows and end the loop, and return.

use std::panic::resume_unwind;
use std::rc::Rc;

use winit::application::ApplicationHandler;
use winit::error::EventLoopError;
use winit::event::WindowEvent;
use winit::event_loop::{ActiveEventLoop, ControlFlow, DeviceEvents, EventLoop};
use winit::platform::windows::EventLoopBuilderExtWindows;
use winit::window::WindowId;

use super::guard::{ActivationGuard, GuardRefused};
use super::loop_core::Core;
use super::retrieval::{RetrievalWatch, WatchRefused};
use super::service::ServiceWindow;
use super::window::Window;
use super::{place, translate};
use crate::event::Event;
use crate::host::{LoopConfig, LoopError, RunReport, ShellHost};

pub(crate) fn run(config: LoopConfig, host: Rc<dyn ShellHost>) -> Result<RunReport, LoopError> {
    // Before the library makes its first window, and dropped last, after every window of the loop is settled, also
    // when the loop unwinds.
    let _guard = ActivationGuard::install().map_err(|refusal| match refusal {
        GuardRefused::AlreadyInstalled => LoopError::AlreadyCreated,
        GuardRefused::System(error) => LoopError::System(error),
    })?;
    // Likewise: what the thread's message calls take out of its queue (`retrieval.rs`).
    let _retrieval = RetrievalWatch::install().map_err(|refusal| match refusal {
        WatchRefused::AlreadyInstalled => LoopError::AlreadyCreated,
        WatchRefused::System(error) => LoopError::System(error),
    })?;
    let event_loop = EventLoop::<()>::with_user_event()
        // Windows needs no particular thread; tests run the loop on one that is bound to a desktop of its own.
        .with_any_thread(true)
        .build()
        .map_err(refused)?;
    let core = Core::new(config, host, event_loop.create_proxy());
    place::lower_own_windows();
    let running = Running {
        service: Some(ServiceWindow::create(&core)?),
        core,
    };
    let core = &running.core;
    if let Some(service) = &running.service {
        core.attach_service(service.hwnd)?;
    }
    let ran = event_loop.run_app(&mut App {
        core: Rc::clone(core),
    });
    let core = Rc::clone(core);
    drop(running);
    let report = core.report();
    if let Some(payload) = core.take_panic() {
        resume_unwind(payload);
    }
    ran.map_err(refused)?;
    Ok(report)
}

fn refused(error: EventLoopError) -> LoopError {
    match error {
        EventLoopError::RecreationAttempt => LoopError::AlreadyCreated,
        EventLoopError::Os(_) => LoopError::System(super::last_error()),
        EventLoopError::NotSupported(_) => LoopError::Library("not supported"),
        EventLoopError::ExitFailure(_) => LoopError::Library("exit failure"),
    }
}

/// Ends what the loop left behind, also when it unwinds. A window a native token still names is hidden and left.
struct Running {
    service: Option<ServiceWindow>,
    core: Rc<Core>,
}

impl Drop for Running {
    fn drop(&mut self) {
        self.core.detach();
        self.core.destroy_windows();
        self.core.forget_windows();
        self.service.take();
    }
}

struct App {
    core: Rc<Core>,
}

impl App {
    /// What only a callback of the library can do.
    fn serve(&self, event_loop: &ActiveEventLoop) {
        while let Some(creation) = self.core.next_creation() {
            match Window::create(&self.core, event_loop, &creation) {
                Ok(window) => self.core.window_created(window, creation.command),
                Err(failure) => self
                    .core
                    .creation_failed(creation.key, creation.command, failure),
            }
        }
        if self.core.has_ended() {
            self.core.destroy_windows();
            // The library waits once more before it looks whether it should end.
            event_loop.set_control_flow(ControlFlow::Poll);
            event_loop.exit();
        }
    }
}

impl ApplicationHandler<()> for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        // It registers for raw input of every mouse and keyboard; each move would wake the loop a second time.
        event_loop.listen_device_events(DeviceEvents::Never);
        event_loop.set_control_flow(ControlFlow::Wait);
        self.core.push(Event::LoopStarted);
    }

    fn user_event(&mut self, event_loop: &ActiveEventLoop, (): ()) {
        self.serve(event_loop);
    }

    fn window_event(&mut self, _: &ActiveEventLoop, id: WindowId, event: WindowEvent) {
        if let Some(window) = self.core.window_by_id(id) {
            translate::window_event(&self.core, &window, event);
        }
    }

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        self.serve(event_loop);
    }
}
