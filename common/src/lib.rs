pub mod brain_service {
    tonic::include_proto!("brain_service");
    pub const FILE_DESCRIPTOR_SET: &[u8] = tonic::include_file_descriptor_set!("communication_descriptor");
}
