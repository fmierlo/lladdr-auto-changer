use std::io::Error;
use std::io::{self, ErrorKind};

use super::sys;
use super::sys::Sys;

pub(crate) trait Socket {
    fn open_local_dgram(&self) -> io::Result<DynOpenSocket>;
}

impl Default for Box<dyn Socket> {
    fn default() -> Box<dyn Socket> {
        Box::new(LibcSocket::default())
    }
}

#[derive(Clone, Debug, Default)]
pub(crate) struct LibcSocket {
    sys: Box<dyn Sys>,
}

impl Socket for LibcSocket {
    fn open_local_dgram(&self) -> io::Result<DynOpenSocket> {
        match self.sys.socket(libc::AF_LOCAL, libc::SOCK_DGRAM, 0) {
            -1 => Err(Error::last_os_error()),
            fd => Ok(Box::new(LibcOpenSocket {
                fd,
                sys: self.sys.clone(),
            })),
        }
    }
}

pub(crate) trait OpenSocket {
    fn get_lladdr(&self, arg: *mut libc::c_void) -> Result<(), Error>;
    fn set_lladdr(&self, arg: *mut libc::c_void) -> Result<(), Error>;
}

pub(crate) type DynOpenSocket = Box<dyn OpenSocket>;

impl Default for Box<dyn OpenSocket> {
    fn default() -> Box<dyn OpenSocket> {
        Box::new(LibcOpenSocket::default())
    }
}

#[derive(Clone, Debug, Default)]
pub(crate) struct LibcOpenSocket {
    fd: libc::c_int,
    sys: Box<dyn Sys>,
}

impl OpenSocket for LibcOpenSocket {
    fn get_lladdr(&self, arg: *mut libc::c_void) -> Result<(), Error> {
        match self.sys.ioctl(self.fd, sys::SIOCGIFLLADDR, arg) {
            0 => Ok(()),
            -1 => Err(Error::last_os_error()),
            err => Err(Error::new(
                ErrorKind::Other,
                format!("LibcOpenSocket.get_lladdr(SIOCGIFLLADDR) -> {err}"),
            )),
        }
    }

    fn set_lladdr(&self, arg: *mut libc::c_void) -> Result<(), Error> {
        match self.sys.ioctl(self.fd, sys::SIOCSIFLLADDR, arg) {
            0 => Ok(()),
            -1 => Err(Error::last_os_error()),
            err => Err(Error::new(
                ErrorKind::Other,
                format!("LibcOpenSocket.set_lladdr(SIOCSIFLLADDR) -> {err}"),
            )),
        }
    }
}

impl Drop for LibcOpenSocket {
    fn drop(&mut self) {
        match self.sys.close(self.fd) {
            0 => (),
            -1 => eprintln!(
                "ERROR: LibcOpenSocket.close() -> {}",
                Error::last_os_error()
            ),
            err => eprintln!("ERROR: LibcOpenSocket.close() -> {err}"),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::macos::ifr;

    use super::*;
    use std::io;
    use sys::{mock::MockSys, Sys};

    // #[test]
    // fn test_socket_new() {
    //     let sys = MockSys::default();
    //     let socket = Socket::new(sys.as_sys());
    //     assert_eq!(socket.sys, sys);
    // }

    // #[test]
    // fn test_socket_open_local_dgram() -> io::Result<()> {
    //     let sys = MockSys::default();
    //     let socket = Socket::new(sys.as_sys());
    //     let fd = socket.open_local_dgram()?;
    //     assert!(!fd.is_null());
    //     Ok(())
    // }

    // #[test]
    // fn test_socket_open_local_dgram_err() -> io::Result<()> {
    //     let mut sys = MockSys::default();
    //     // Set the error code to -1
    //     sys.set_last_os_error(ErrorCode::last_os_error());
    //     let socket = Socket::new(sys.as_sys());
    //     let fd = socket.open_local_dgram()?;
    //     assert_eq!(fd, -1);
    //     Ok(())
    // }

    #[test]
    fn test_local_dgram_socket_get_lladdr() -> io::Result<()> {
        // Given
        let name = "en";
        let expected_mac_address = "00:11:22:33:44:55";

        let sys = MockSys::default().with_nic(name, expected_mac_address);
        let socket = LibcSocket { sys: sys.as_sys() };

        let mut ifr = ifr::new();
        ifr::set_name(&mut ifr, name);
        // When
        socket
            .open_local_dgram()?
            .get_lladdr(ifr::to_c_void_ptr(&mut ifr))?;
        let mac_address = ifr::get_mac_address(&ifr);
        // Then
        assert_eq!(mac_address, expected_mac_address);
        Ok(())
    }

    // #[test]
    // fn test_local_dgram_socket_get_lladdr_err() -> io::Result<()> {
    //     let mut sys = MockSys::default();
    //     // Set the error code to -1
    //     sys.set_last_os_error(ErrorCode::last_os_error());
    //     let socket = Socket::new(sys.as_sys());
    //     let fd = socket.open_local_dgram()?;
    //     assert_eq!(
    //         socket.get_lladdr(&mut [0; 16])?,
    //         Err(Error::last_os_error())
    //     );
    //     Ok(())
    // }

    #[test]
    fn test_local_dgram_socket_set_lladdr() -> io::Result<()> {
        // Given
        let name = "en";
        let mac_address = "00:11:22:33:44:55";

        let sys = MockSys::default();
        let socket = LibcSocket { sys: sys.as_sys() };

        let mut ifr = ifr::new();
        ifr::set_name(&mut ifr, name);
        ifr::set_mac_address(&mut ifr, mac_address);
        // When
        socket
            .open_local_dgram()?
            .set_lladdr(ifr::to_c_void_ptr(&mut ifr))?;
        // Then
        assert!(sys.has_nic(&name, &mac_address));
        Ok(())
    }

    // #[test]
    // fn test_local_dgram_socket_set_lladdr_err() -> io::Result<()> {
    //     let mut sys = MockSys::default();
    //     // Set the error code to -1
    //     sys.set_last_os_error(ErrorCode::last_os_error());
    //     let socket = Socket::new(sys.as_sys());
    //     let fd = socket.open_local_dgram()?;
    //     assert_eq!(
    //         socket.set_lladdr(&mut [0; 16])?,
    //         Err(Error::last_os_error())
    //     );
    //     Ok(())
    // }

    // #[test]
    // fn test_socket_close() {
    //     let sys = MockSys::default();
    //     // Create a dummy local dgram socket
    //     let socket = Socket::new(sys.as_sys());
    //     let fd = socket.open_local_dgram()?;
    //     assert!(!fd.is_null());
    //     drop(socket); // Close the socket
    // }
}

#[cfg(test)]
pub(crate) mod mock {
    use crate::macos::ifr;

    use super::{DynOpenSocket, OpenSocket, Socket};
    use std::{cell::RefCell, collections::HashMap, io::{self, Error}, rc::Rc};

    type KeyValue = RefCell<HashMap<String, String>>;

    #[derive(Clone, Debug, Default)]
    pub(crate) struct MockSocket {
        kv: Rc<KeyValue>,
    }

    impl MockSocket {
        pub(crate) fn as_socket(&self) -> Box<dyn Socket> {
            Box::new(Self {
                kv: Rc::clone(&self.kv),
            })
        }
        pub(crate) fn with_nic(self, name: &str, mac_address: &str) -> Self {
            self.set_nic(name, mac_address);
            self
        }

        pub(crate) fn set_nic(&self, name: &str, mac_address: &str) {
            self.kv
                .borrow_mut()
                .insert(name.to_string(), mac_address.to_string());
        }

        pub(crate) fn has_nic(&self, name: &str, expected_mac_address: &str) -> bool {
            match self.kv.borrow().get(name) {
                Some(mac_address) => mac_address == expected_mac_address,
                None => false,
            }
        }
    }

    impl Socket for MockSocket {
        fn open_local_dgram(&self) -> io::Result<DynOpenSocket> {
            eprintln!("MockSocket.open_local_dgram()");
            Ok(Box::new(MockOpenSocket {
                kv: Rc::clone(&self.kv),
            }))
        }
    }
    #[derive(Clone, Debug, Default)]
    pub(crate) struct MockOpenSocket {
        kv: Rc<KeyValue>,
    }

    impl OpenSocket for MockOpenSocket {
        fn get_lladdr(&self, arg: *mut libc::c_void) -> Result<(), Error> {
            let ifr = ifr::from_c_void_ptr(arg);
            let name = ifr::get_name(ifr);
            match self.kv.borrow().get(name) {
                Some(mac_address) => {
                    eprintln!("MockOpenSocket.get_lladdr({name}) -> {mac_address})");
                    ifr::set_mac_address(ifr, &mac_address)
                }
                _ => {}
            };
            Ok(())
        }

        fn set_lladdr(&self, arg: *mut libc::c_void) -> Result<(), Error> {
            let ifr = ifr::from_c_void_ptr(arg);
            let name = ifr::get_name(ifr);
            let mac_address = ifr::get_mac_address(ifr);
            eprintln!("MockOpenSocket.set_lladdr({name}, {mac_address})");
            self.kv
                .borrow_mut()
                .insert(name.to_string(), mac_address.to_string());
            Ok(())
        }
    }
}