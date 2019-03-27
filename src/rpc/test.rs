use super::{
    RPC,
    Portmap,
    Mount,
    NFS,
    RPCProgram,
    RPCProcedure,
    RPCCall,
};

fn set_rpc_version_2(buffer: &mut [u8]) {
    buffer[11] = 0x02;
}

#[test]
fn it_fails_marshalling_rpc_v1() {
    let buffer = [0x00; 72];
    assert_eq!(match RPC::unmarshall_message(&buffer) {
        Ok((_, _)) => "success",
        Err(err) => err,
    }, "Only RPC Version 2 is supported");
}

#[test]
fn it_can_marshalling_rpc_v2() {
    let mut buffer = [0x00; 72];
    set_rpc_version_2(&mut buffer);

    assert_eq!(match RPC::unmarshall_message(&buffer) {
        Ok((call, _)) => call.rpc_version,
        Err(_) => [0x00; 4],
    }, [0x00, 0x00, 0x00, 0x02]);
}

#[test]
fn it_can_unmarshal_message_into_rpc_call() {
    let mut buffer = [0x00; 76];
    set_rpc_version_2(&mut buffer);

    assert_eq!(match RPC::unmarshall_message(&buffer) {
        Ok((_, parameters)) => parameters.len(),
        Err(_) => 0,
    }, 16);

    // Verify that our RPC packages is exactly 60 bytes
    let mut buffer = [0x00; 90];
    set_rpc_version_2(&mut buffer);

    assert_eq!(match RPC::unmarshall_message(&buffer) {
        Ok((_, parameters)) => parameters.len(),
        Err(_) => 0,
    }, 30);
}

#[test]
fn it_can_match_programs_and_procedures() {
    // Portmap
    let mut buffer = [0x00; 72];
    set_rpc_version_2(&mut buffer);
    buffer[13] = 0x01;
    buffer[14] = 0x86;
    buffer[15] = 0xa0;
    buffer[23] = 0x03;

    // Assert ergonomics
    const ERRORED_CASE: (RPCProgram, RPCProcedure) = (
        RPCProgram::Unknown,
        RPCProcedure::Unknown
    );

    fn assert_unpacking(
        expected: (RPCProgram, RPCProcedure),
        buffer: &mut [u8]
    ) {
        assert_eq!(match RPC::unmarshall_message(buffer) {
            Ok(res) => (res.0.program, res.0.procedure),
            Err(_) => ERRORED_CASE,
        }, expected);
    }

    assert_unpacking((RPCProgram::Portmap, RPCProcedure::Getport), &mut buffer);

    buffer[15] = 0xa5;
    buffer[23] = 0x05;
    assert_unpacking((RPCProgram::Mount, RPCProcedure::Export), &mut buffer);

    buffer[23] = 0x01;
    assert_unpacking((RPCProgram::Mount, RPCProcedure::Mnt), &mut buffer);

    // Test NFS program
    buffer[15] = 0xa3;
    buffer[23] = 0x04;
    assert_unpacking((RPCProgram::NFS, RPCProcedure::Lookup), &mut buffer);

    buffer[23] = 0x06;
    assert_unpacking((RPCProgram::NFS, RPCProcedure::Read), &mut buffer);

    buffer[23] = 0x01;
    assert_unpacking((RPCProgram::NFS, RPCProcedure::Getattr), &mut buffer);

    buffer[23] = 0xff;
    assert_unpacking((RPCProgram::NFS, RPCProcedure::Unknown), &mut buffer);

    // Test fallback
    buffer[15] = 0xDD;
    buffer[23] = 0xDD;
    assert_unpacking((RPCProgram::Unknown, RPCProcedure::Unknown), &mut buffer);
}

#[test]
fn it_will_fail_if_message_type_is_reply() {
    let mut buffer = [0x00; 76];
    buffer[7] = 0x01;

    assert_eq!(RPC::unmarshall_message(&buffer), Err("Server received a reply"));
}

#[test]
fn it_will_unmarshall_into_rpc_enum() {
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

    let program = RPC::unmarshall(&buffer);
    assert_eq!(match program {
        RPC::Portmap(_rpc_call, program) => {
            match program {
                Portmap::Program::Getport(procedure) => {
                    match procedure {
                        Portmap::Procedure::Mount(portmap) => {
                            assert_eq!(portmap.port, [0x00, 0x01, 0xa4, 0x37]);
                            true
                        },
                        _ => false,
                    }
                },
                _ => false,
            }
        },
        _ => false,
    }, true);
}
