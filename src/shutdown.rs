pub(crate) struct Shutdown {
    sender: tokio::sync::mpsc::Sender<bool>,
    receiver: tokio::sync::mpsc::Receiver<bool>,
}

impl Shutdown {
    pub fn new() -> Shutdown {
        let (sender, receiver) = tokio::sync::mpsc::channel(1);
        Shutdown { sender, receiver }
    }

    pub fn watch(&self) -> Watcher {
        Watcher(self.sender.clone())
    }

    pub async fn shutdown(self) {
        let Shutdown {
            sender,
            mut receiver,
        } = self;
        drop(sender);
        receiver.recv().await;
    }
}

pub(crate) struct Watcher(tokio::sync::mpsc::Sender<bool>);
