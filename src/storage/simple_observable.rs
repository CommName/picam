use tokio::{sync::broadcast::Receiver, sync::broadcast::*};

use super::{ObservableStorage, SimpleStorage};



#[derive(Clone)]
pub struct SimpleObservable<T, D> {
    storage: T,
    broadcast: Sender<D>
}

impl<T,D> SimpleObservable<T, D> 
where D: Clone 
{

    pub fn new(storage: T) -> Self {
        Self {
            storage,
            broadcast: channel(1).0
        }
    }
}

#[async_trait::async_trait]
impl<T,D> SimpleStorage<D> for SimpleObservable<T, D>
where T: SimpleStorage<D> + Send + Sync,
D: Clone + Send + Sync {

    async fn get(&self) -> D {
        self.storage.get().await
    }

    async fn set(&self, value: &D) {
        self.storage.set(value).await;
        let _ = self.broadcast.send(value.clone());
    }
}

#[async_trait::async_trait]
impl<T, D> super::Observable<D> for SimpleObservable<T, D> 
where T: SimpleStorage<D> + Send + Sync,
      D: Clone + Send + Sync
{
    async fn subscribe(&self) -> Receiver<D> {
        self.broadcast.subscribe()
    }
}

impl<T,D> ObservableStorage<D> for SimpleObservable<T, D> 
where T: SimpleStorage<D> + Send + Sync,
      D: Clone + Send + Sync {

}