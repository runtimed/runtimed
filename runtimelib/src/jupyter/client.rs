use tokio::time::{timeout, Duration};

use anyhow::anyhow;
use anyhow::Error;
use ring::hmac;
use serde::{Deserialize, Serialize};
use serde_json::json;
use serde_json::Value;
use uuid::Uuid;
use zeromq;
use zeromq::{Socket, SocketRecv, SocketSend, SocketType, ZmqMessage};

use crate::jupyter::content::shell::{
    ExecuteReply, ExecuteRequest, KernelInfoReply, KernelInfoRequest,
};
use crate::jupyter::message::Message;
use crate::jupyter::request::Request;
use crate::jupyter::response::Response;

#[derive(Serialize, Clone)]
pub struct JupyterEnvironment {
    process: String,
    argv: Vec<String>,
    display_name: String,
    language: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct JupyterRuntime {
    #[serde(default)]
    pub id: Uuid,
    pub shell_port: u16,
    pub iopub_port: u16,
    pub stdin_port: u16,
    pub control_port: u16,
    pub hb_port: u16,
    pub kernel_name: String,
    pub ip: String,
    key: String,
    pub transport: String, // TODO: Enumify with tcp, ipc
    signature_scheme: String,
    // We'll track the connection file path here as well
    #[serde(default)]
    pub connection_file: String,
    #[serde(default)]
    pub state: String, // TODO: Use an enum
    pub kernel_info: KernelInfoReply,
}

impl JupyterRuntime {
    pub async fn attach(&self) -> Result<JupyterClient, Error> {
        let mut iopub = zeromq::SubSocket::new();
        match iopub.subscribe("").await {
            Ok(_) => (),
            Err(e) => return Err(anyhow!("Error subscribing to iopub: {}", e)),
        }

        iopub
            .connect(&format!(
                "{}://{}:{}",
                self.transport, self.ip, self.iopub_port
            ))
            .await?;

        let mut shell = zeromq::DealerSocket::new();
        shell
            .connect(&format!(
                "{}://{}:{}",
                self.transport, self.ip, self.shell_port
            ))
            .await?;

        let mut stdin = zeromq::DealerSocket::new();
        stdin
            .connect(&format!(
                "{}://{}:{}",
                self.transport, self.ip, self.stdin_port
            ))
            .await?;

        let mut control = zeromq::DealerSocket::new();
        control
            .connect(&format!(
                "{}://{}:{}",
                self.transport, self.ip, self.control_port
            ))
            .await?;

        let mut heartbeat = zeromq::ReqSocket::new();
        heartbeat
            .connect(&format!(
                "{}://{}:{}",
                self.transport, self.ip, self.hb_port
            ))
            .await?;

        Ok(JupyterClient {
            key: self.key.clone(),
            iopub,
            shell,
            stdin,
            control,
            heartbeat,
        })
    }
}

pub struct JupyterClient {
    key: String,
    pub(crate) shell: zeromq::DealerSocket,
    pub(crate) iopub: zeromq::SubSocket,
    pub(crate) stdin: zeromq::DealerSocket,
    pub(crate) control: zeromq::DealerSocket,
    pub(crate) heartbeat: zeromq::ReqSocket,
}

impl JupyterClient {
    pub async fn detach(self) -> Result<(), Error> {
        let timeout_duration = Duration::from_millis(60);

        let close_sockets = async {
            let _ = tokio::join!(
                self.shell.close(),
                self.iopub.close(),
                self.stdin.close(),
                self.control.close(),
                self.heartbeat.close(),
            );
        };

        match timeout(timeout_duration, close_sockets).await {
            Ok(_) => Ok(()),
            Err(_) => Err(anyhow!("Timeout reached while closing sockets.")),
        }
    }

    pub async fn kernel_info(&mut self) -> Result<Message<KernelInfoReply>, Error> {
        let request: Request = KernelInfoRequest::new().into();
        let response: Response = self.send(request).await?;

        match response {
            Response::KernelInfo(reply) => Ok(reply),
            _ => Err(anyhow!("Unexpected response from kernel_info")),
        }
    }

    pub async fn send(&mut self, message: Request) -> Result<Response, Error> {
        let wire_protocol = message.into_wire_protocol(&self.key);

        let zmq_message: ZmqMessage = wire_protocol.into();
        self.shell.send(zmq_message).await?;
        let response: Response = self.shell.recv().await?.into();

        Ok(response)
    }

    pub async fn next_io(&mut self) -> Result<Response, Error> {
        let response: Response = self.iopub.recv().await?.into();
        Ok(response)
    }

    pub async fn run_code(&mut self, code: String) -> Result<Message<ExecuteReply>, Error> {
        let request: Request = ExecuteRequest::new(code).into();
        let response: Response = self.send(request).await?;

        match response {
            Response::Execute(reply) => {
                println!("Execution result: {:?}", reply);
                Ok(reply)
            }
            _ => Err(anyhow!("Unexpected response from execute")),
        }
    }
}
