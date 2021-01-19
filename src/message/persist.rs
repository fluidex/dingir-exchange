use crate::database;
use super::consumer; //crate::message::consumer
use rdkafka::consumer::Consumer;
use serde::Deserialize;

//An simple handler, just persist it by DatabaseWriter
impl<'a, 'c, C, U> TypedMessageHandler<'c, C> for &'a database::DatabaseWriter<U>
where 
    U: 'static + std::fmt::Debug + Clone + Send + for<'de> Deserialize<'de>,
    C: Consumer + Sync,
{
    type DataType = U;
    async fn on_message(&self, msg : U, _cr :&'c C)
    {
        self.append(U);
    }
    async fn on_no_msg(&self, _cr: &'c C){} //do nothing
}
