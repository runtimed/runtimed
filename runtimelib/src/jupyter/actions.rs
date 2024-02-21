use std::fmt::Debug;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll, Waker};

use tokio::sync::{mpsc, Mutex};

use crate::jupyter::handlers::Handler;
use crate::jupyter::iopub_content::status::KernelStatus;
use crate::jupyter::request::Request;
use crate::jupyter::response::Response;

#[derive(Debug, PartialEq)]
pub enum ExpectedReplyType {
    KernelInfo,
    ExecuteReply,
    None,
}

impl From<&Request> for ExpectedReplyType {
    fn from(request: &Request) -> Self {
        match request {
            Request::KernelInfo(_) => ExpectedReplyType::KernelInfo,
            Request::Execute(_) => ExpectedReplyType::ExecuteReply,
        }
    }
}

impl From<&Response> for ExpectedReplyType {
    fn from(response: &Response) -> Self {
        match response {
            Response::KernelInfo(_) => ExpectedReplyType::KernelInfo,
            Response::Execute(_) => ExpectedReplyType::ExecuteReply,
            _ => ExpectedReplyType::None,
        }
    }
}

#[derive(Debug)]
struct ActionState {
    completed: bool,
    waker: Option<Waker>,
}

#[derive(Debug)]
pub struct Action {
    pub request: Request,
    state: Arc<Mutex<ActionState>>,
}

impl Action {
    pub fn new(
        request: Request,
        handlers: Vec<Arc<Mutex<dyn Handler>>>,
        msg_rx: mpsc::Receiver<Response>,
    ) -> Self {
        let action_state = Arc::new(Mutex::new(ActionState {
            completed: false,
            waker: None,
        }));
        let expected_reply = ExpectedReplyType::from(&request);
        // spawn background task for listening
        tokio::spawn(Action::listen(
            msg_rx,
            expected_reply,
            handlers,
            action_state.clone(),
        ));
        Action {
            request,
            state: action_state,
        }
    }

    async fn listen(
        mut msg_rx: mpsc::Receiver<Response>,
        expected_reply: ExpectedReplyType,
        handlers: Vec<Arc<Mutex<dyn Handler>>>,
        action_state: Arc<Mutex<ActionState>>,
    ) {
        // We "finish" this background task when kernel idle and expected reply (if relevant) seen
        let mut kernel_idle = false;
        let mut expected_reply_seen = match expected_reply {
            ExpectedReplyType::KernelInfo => false,
            ExpectedReplyType::ExecuteReply => false,
            ExpectedReplyType::None => true,
        };
        while let Some(response) = msg_rx.recv().await {
            for handler_arc in &handlers {
                let mut handler = handler_arc.lock().await;
                handler.handle(&response).await;
            }
            match response {
                Response::Status(status) => {
                    if status.content.execution_state == KernelStatus::Idle {
                        kernel_idle = true;
                    }
                }
                _ => {
                    if expected_reply == ExpectedReplyType::from(&response) {
                        expected_reply_seen = true;
                    }
                }
            }
            if kernel_idle && expected_reply_seen {
                let mut state = action_state.lock().await;
                state.completed = true;
                if let Some(waker) = state.waker.take() {
                    waker.wake();
                }
                break;
            }
        }
    }
}

impl Future for Action {
    type Output = ();

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let mut state = match self.state.try_lock() {
            Ok(state) => state,
            Err(_) => {
                // If we can't get the lock, it means the background task is still running
                // and we need to wait for it to complete
                return Poll::Pending;
            }
        };
        if state.completed {
            Poll::Ready(())
        } else {
            state.waker = Some(cx.waker().clone());
            Poll::Pending
        }
    }
}
