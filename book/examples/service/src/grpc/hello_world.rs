use crate::grpc::hello_world::greeter_server::Greeter;
use tonic::{Request, Response, Status};
use tracing::{info, instrument};

tonic::include_proto!("helloworld");

pub const FILE_DESCRIPTOR_SET: &[u8] = tonic::include_file_descriptor_set!("helloworld_descriptor");

pub struct MyGreeter;

#[tonic::async_trait]
impl Greeter for MyGreeter {
    #[instrument(skip_all)]
    async fn say_hello(
        &self,
        request: Request<HelloRequest>,
    ) -> Result<Response<HelloReply>, Status> {
        let name = request.into_inner().name;
        info!("Saying hello to {}", name);

        let reply = HelloReply {
            message: format!("Hello {}!", name),
        };

        Ok(Response::new(reply))
    }
}
