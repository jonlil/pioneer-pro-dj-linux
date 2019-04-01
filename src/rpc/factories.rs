use super::RPC;

fn set_rpc_version_2(buffer: &mut [u8]) {
    buffer[11] = 0x02;
}

pub fn generate_rpc_mount_procedure() -> RPC {
    let mut buffer = [0x00; 76];
    set_rpc_version_2(&mut buffer);
    buffer[13] = 0x01;
    buffer[14] = 0x86;
    buffer[15] = 0xa0;
    buffer[23] = 0x03;

    buffer[60] = 0x00;
    buffer[61] = 0x01;
    buffer[62] = 0x86;
    buffer[63] = 0xa5;

    buffer[72] = 0x00;
    buffer[73] = 0x01;
    buffer[74] = 0xa4;
    buffer[75] = 0x37;

    RPC::unmarshall(&buffer)
}
