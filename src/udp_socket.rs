use crate::{
    error::Error,
    socket::{Socket, SocketFamily, SocketProtocol, SocketType, SplitSocketHandle},
};
use no_std_net::{SocketAddr, ToSocketAddrs};

/// A socket that sends and receives UDP messages
pub struct UdpSocket {
    inner: Socket,
}

macro_rules! impl_receive_from {
    () => {
        /// Try to fill the given buffer with received data.
        /// The part of the buffer that was filled is returned together with the address of the source of the message.
        pub async fn receive_from<'buf>(
            &self,
            buf: &'buf mut [u8],
        ) -> Result<(&'buf mut [u8], SocketAddr), Error> {
            let (received_len, addr) = self.socket().receive_from(buf).await?;
            Ok((&mut buf[..received_len], addr))
        }
    };
}

macro_rules! impl_send_to {
    () => {
        /// Send the given buffer to the given address
        pub async fn send_to(&self, buf: &[u8], addr: SocketAddr) -> Result<(), Error> {
            self.socket().send_to(buf, addr).await.map(|_| ())
        }
    };
}

impl UdpSocket {
    /// Bind a new socket to the given address
    pub async fn bind(addr: impl ToSocketAddrs) -> Result<Self, Error> {
        let mut last_error = None;

        let addrs = addr.to_socket_addrs().unwrap();
        let mut socketv4 = None;
        let mut socketv6 = None;

        for addr in addrs {
            let socket = match addr {
                no_std_net::SocketAddr::V4(_) => match socketv4 {
                    Some(_) => &mut socketv4,
                    None => {
                        socketv4 = Some(
                            Socket::create(
                                SocketFamily::Ipv4,
                                SocketType::Datagram,
                                SocketProtocol::Udp,
                            )
                            .await?,
                        );
                        &mut socketv4
                    }
                },
                no_std_net::SocketAddr::V6(_) => match socketv6 {
                    Some(_) => &mut socketv6,
                    None => {
                        socketv6 = Some(
                            Socket::create(
                                SocketFamily::Ipv6,
                                SocketType::Datagram,
                                SocketProtocol::Udp,
                            )
                            .await?,
                        );
                        &mut socketv6
                    }
                },
            };

            match socket.as_mut().unwrap().bind(addr).await {
                Ok(_) => {
                    return Ok(UdpSocket {
                        inner: socket.take().unwrap(),
                    })
                }
                Err(e) => {
                    last_error = Some(e);
                }
            }
        }

        Err(last_error.take().unwrap())
    }

    /// Get the raw underlying file descriptor
    pub fn as_raw_fd(&self) -> i32 {
        self.inner.as_raw_fd()
    }

    fn socket(&self) -> &Socket {
        &self.inner
    }

    /// Split the socket into an owned read and write half
    pub fn split_owned(self) -> (OwnedUdpReceiveSocket, OwnedUdpSendSocket) {
        let (read_split, write_split) = self.inner.split();

        (
            OwnedUdpReceiveSocket { socket: read_split },
            OwnedUdpSendSocket {
                socket: write_split,
            },
        )
    }

    /// Split the socket into a borrowed read and write half
    pub fn split(&self) -> (UdpReceiveSocket<'_>, UdpSendSocket<'_>) {
        (
            UdpReceiveSocket { socket: self },
            UdpSendSocket { socket: self },
        )
    }

    impl_receive_from!();
    impl_send_to!();

    /// Deactivates the socket and the LTE link.
    /// A normal drop will do the same thing, but blocking.
    pub async fn deactivate(self) -> Result<(), Error> {
        self.inner.deactivate().await?;
        Ok(())
    }
}

pub struct UdpReceiveSocket<'a> {
    socket: &'a UdpSocket,
}

impl<'a> UdpReceiveSocket<'a> {
    fn socket(&self) -> &Socket {
        &self.socket.inner
    }

    impl_receive_from!();
}

pub struct UdpSendSocket<'a> {
    socket: &'a UdpSocket,
}

impl<'a> UdpSendSocket<'a> {
    fn socket(&self) -> &Socket {
        &self.socket.inner
    }

    impl_send_to!();
}

pub struct OwnedUdpReceiveSocket {
    socket: SplitSocketHandle,
}

impl OwnedUdpReceiveSocket {
    fn socket(&self) -> &Socket {
        &self.socket
    }

    impl_receive_from!();

    /// Deactivates the socket and the LTE link.
    /// A normal drop will do the same thing, but blocking.
    pub async fn deactivate(self) -> Result<(), Error> {
        self.socket.deactivate().await?;
        Ok(())
    }
}

pub struct OwnedUdpSendSocket {
    socket: SplitSocketHandle,
}

impl OwnedUdpSendSocket {
    fn socket(&self) -> &Socket {
        &self.socket
    }

    impl_send_to!();

    /// Deactivates the socket and the LTE link.
    /// A normal drop will do the same thing, but blocking.
    pub async fn deactivate(self) -> Result<(), Error> {
        self.socket.deactivate().await?;
        Ok(())
    }
}
