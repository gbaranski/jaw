# Jaw: the remote shell

Jaw is a remote shell that uses UDP as a transport protocol.

It is currently just a proof-of-concept, and a result of my play with UDP.

# Flow

![flow image](./docs/flow.svg)

# Features

- [x] Running remote shell over UDP.
- [x] Multiple session support.
- [x] True color support.
- [ ] NAT hole punching 
- [ ] IP address change on the fly.
- [ ] Encryption.
- [ ] Authentication.
- [ ] Local echo.
- [ ] Adjustable frame rate.
- [ ] Deleting old sessions.
- [ ] Performance comparsion to SSH.

# Frame

## Client -> Server

### NewSession

Frame with empty body, used to create a new session on a server. Server must respond with [NewSessionAck](#NewSessionAck)

### Write

Frame with array of bytes, server writes those bytes to a child process.

## Server -> Client

### NewSessionAck

Frame contains a newly created Session ID. Sent in a response to [NewSession](#NewSession)

### Write

Frame with array of bytes, client writes those bytes to stdout.
