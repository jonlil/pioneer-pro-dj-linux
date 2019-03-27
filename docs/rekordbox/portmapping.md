## GETPORT

### GETPORT Call (Player to application)

|                                Remote procedure call                               |
|-------------|--------------|-------------|-------------|-------------|-------------|
| XID         | Message type | RPC Ver.    | Prog.       | Prog. Ver.  | Procedure   |
|-------------|--------------|-------------|-------------|-------------|-------------|
| 00:00:00:01 | 00:00:00:00  | 00:00:00:02 | 00:01:86:a0 | 00:00:00:02 | 00:00:00:03 |

|                                           Credentials                                            |         Verifier          |
|-------------|-------------|-------------|--------------+-------------+-------------+-------------+-------------+-------------|
| Flavor      | Length      | Stamp       | Machine name | UID         | GID         | AUX GID     | Flavor      | Length      |
|-------------|-------------|-------------|--------------+-------------+-------------+-------------+-------------+-------------|
| 00:00:00:01 | 00:00:00:14 | 96:7b:87:03 | 00:00:00:00  | 00:00:00:00 | 00:00:00:00 | 00:00:00:00 | 00:00:00:00 | 00:00:00:00 |

| Portmap: GETPORT Call MOUNT                           |
|-------------+-------------+-------------+-------------|
| Program     | Version     | Protocol    | Port        |
|-------------+-------------+-------------+-------------+
| 00:01:86:a5 | 00:00:00:01 | 00:00:00:11 | 00:00:00:00 |

### GETPORT Reply (Application to player)

| Reply                                   |         Verifier          |              |
|-------------|-------------|-------------+-------------+-------------+--------------|
| XID         | Message t.  | Reply state | Flavor      | Length      | Accept state |
|-------------+-------------+-------------+-------------+-------------+--------------|
| 00:00:00:01 | 00:00:00:01 | 00:00:00:00 | 00:00:00:00 | 00:00:00:00 | 00:00:00:00  |

| Portmap: Reply |
|----------------|
| Port           |
|----------------|
| 00:00:a4:37    |

=========

For the communication to work the server should allocate SocketAddr
and in the portmap reply that port address should be transmitted.

Eg:

PORTMAP Reply: port 42039
EXPORT Call dstport 42039


## Export

### EXPORT Call (Player to application)

|                                Remote procedure call                               |
|-------------|--------------|-------------|-------------|-------------|-------------|
| XID         | Message type | RPC Ver.    | Prog.       | Prog. Ver.  | Procedure   |
|-------------|--------------|-------------|-------------|-------------|-------------|
| 00:00:00:02 | 00:00:00:00  | 00:00:00:02 | 00:01:86:a5 | 00:00:00:01 | 00:00:00:05 |

|                                           Credentials                                            |         Verifier          |
|-------------|-------------|-------------|--------------|-----------------------------------------|-------------|-------------|
| Flavor      | Length      | Stamp       | Machine name | UID         | GID         | AUX GID     | Flavor      | Length      |
|-------------|-------------|-------------|--------------|-------------|-------------|-------------|-------------|-------------|
| 00:00:00:01 | 00:00:00:14 | 99:22:e1:12 | 00:00:00:00  | 00:00:00:00 | 00:00:00:00 | 00:00:00:00 | 00:00:00:00 | 00:00:00:00 |


Frame 566: 102 bytes on wire (816 bits), 102 bytes captured (816 bits) on interface 0
Ethernet II, Src: PioneerD_04:1d:e6 (c8:3d:fc:04:1d:e6), Dst: Apple_35:bc:4d (ac:87:a3:35:bc:4d)
Internet Protocol Version 4, Src: 169.254.29.230, Dst: 169.254.37.116
User Datagram Protocol, Src Port: 4056, Dst Port: 42039
Remote Procedure Call, Type:Call XID:0x00000002
    XID: 0x00000002 (2)
    Message Type: Call (0)
    RPC Version: 2
    Program: MOUNT (100005)
    Program Version: 1
    Procedure: EXPORT (5)
    [The reply to this request is in frame 568]
    Credentials
        Flavor: AUTH_UNIX (1)
        Length: 20
        Stamp: 0x9922e112
        Machine Name: <EMPTY>
            length: 0
            contents: <EMPTY>
        UID: 0
        GID: 0
        Auxiliary GIDs (0)
    Verifier
        Flavor: AUTH_NULL (0)
        Length: 0
Mount Service
    [Program Version: 1]
    [V1 Procedure: EXPORT (5)]

=====

### EXPORT Reply (Application to player)

|                                Remote procedure call                                |
|----------------------------------------------------------------------+--------------|
| Type:Reply                               |          Verifer          |              |
|-------------+--------------+-------------+-------------+-------------+--------------|
| XID         | Message type | Reply state | Flavor      | Length      | Accept State |
|-------------+--------------+-------------+-------------+-------------+--------------|
| 00:00:00:02 | 00:00:00:01  | 00:00:00:00 | 00:00:00:00 | 00:00:00:00 | 00:00:00:00  |

| Mount service |
|---------------|
| Value follows |
|---------------|
| 00:00:00:00   |
