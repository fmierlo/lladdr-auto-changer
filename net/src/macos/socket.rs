use std::fmt::Debug;

use std::ops::Deref;

use crate::{str_from_ptr_or_empty, Result};

use super::sys::{self, BoxSys};

pub(crate) trait Socket: Debug {
    fn open_local_dgram(&self) -> Result<Box<dyn OpenSocket + '_>>;
}

#[derive(Debug, Default)]
pub(crate) struct BoxSocket(pub(crate) Box<dyn Socket>);

impl Default for Box<dyn Socket> {
    fn default() -> Self {
        Box::new(LibcSocket::default())
    }
}

impl Deref for BoxSocket {
    type Target = Box<dyn Socket>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[derive(Debug, Default)]
pub(crate) struct LibcSocket(BoxSys);

impl Deref for LibcSocket {
    type Target = BoxSys;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<'a> Socket for LibcSocket {
    fn open_local_dgram(&self) -> Result<Box<dyn OpenSocket + '_>> {
        match self.socket(libc::AF_LOCAL, libc::SOCK_DGRAM, 0) {
            fd if fd >= 0 => Ok(Box::new(LibcOpenSocket { fd, sys: &self })),
            ret => Err(format!(
                "LibcSocket.socket(AF_LOCAL, SOCK_DGRAM, 0) -> ret={ret} errno={} err={}",
                self.errno(),
                str_from_ptr_or_empty(self.strerror())
            )
            .into()),
        }
    }
}

pub(crate) trait OpenSocket {
    fn get_lladdr(&self, arg: *mut libc::c_void) -> Result<()>;
    fn set_lladdr(&self, arg: *mut libc::c_void) -> Result<()>;
}

pub(crate) struct LibcOpenSocket<'a> {
    fd: libc::c_int,
    sys: &'a BoxSys,
}

impl<'a> Deref for LibcOpenSocket<'a> {
    type Target = &'a BoxSys;

    fn deref(&self) -> &Self::Target {
        &self.sys
    }
}

impl<'a> OpenSocket for LibcOpenSocket<'a> {
    fn get_lladdr(&self, arg: *mut libc::c_void) -> Result<()> {
        match self.ioctl(self.fd, sys::SIOCGIFLLADDR, arg) {
            0 => Ok(()),
            ret => Err(format!(
                "LibcOpenSocket.ioctl(fd={}, SIOCGIFLLADDR) -> ret={ret} errno={} err={}",
                self.fd,
                self.errno(),
                str_from_ptr_or_empty(self.strerror())
            )
            .into()),
        }
    }

    fn set_lladdr(&self, arg: *mut libc::c_void) -> Result<()> {
        match self.ioctl(self.fd, sys::SIOCSIFLLADDR, arg) {
            0 => Ok(()),
            ret => Err(format!(
                "LibcOpenSocket.ioctl(fd={}, SIOCSIFLLADDR) -> ret={ret} errno={} err={}",
                self.fd,
                self.errno(),
                str_from_ptr_or_empty(self.strerror())
            )
            .into()),
        }
    }
}

impl<'a> Drop for LibcOpenSocket<'a> {
    fn drop(&mut self) {
        match self.close(self.fd) {
            0 => (),
            ret => eprintln!(
                "ERROR: LibcOpenSocket.close(fd={}) -> ret={ret} errno={} err={}",
                self.fd,
                self.errno(),
                str_from_ptr_or_empty(self.strerror())
            )
            .into(),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        macos::ifreq::{self},
        IfName, LLAddr,
    };

    use super::*;

    use sys::mock::MockSys;

    impl<'a> LibcSocket {
        fn new(sys: &MockSys) -> LibcSocket {
            LibcSocket(BoxSys(Box::new(sys.clone())))
        }
    }

    // #[test]
    // fn test_socket_new() {
    //     let sys = MockSys::default();
    //     let socket = Socket::new(sys.as_sys());
    //     assert_eq!(socket.sys, sys);
    // }

    // #[test]
    // fn test_socket_open_local_dgram() -> Result<()> {
    //     let sys = MockSys::default();
    //     let socket = Socket::new(sys.as_sys());
    //     let fd = socket.open_local_dgram()?;
    //     assert!(!fd.is_null());
    //     Ok(())
    // }

    // #[test]
    // fn test_socket_open_local_dgram_err() -> Result<()> {
    //     let mut sys = MockSys::default();
    //     // Set the error code to -1
    //     sys.set_last_os_error(ErrorCode::last_os_error());
    //     let socket = Socket::new(sys.as_sys());
    //     let fd = socket.open_local_dgram()?;
    //     assert_eq!(fd, -1);
    //     Ok(())
    // }

    #[test]
    fn test_local_dgram_socket_get_lladdr() -> Result<()> {
        // Given
        let ifname: IfName = "en".try_into()?;
        let expected_lladdr: LLAddr = "00:11:22:33:44:55".parse()?;
        let sys = MockSys::default().with_nic(ifname, expected_lladdr);
        let mut ifreq = ifreq::new();
        ifreq::set_name(&mut ifreq, &ifname);
        // When
        LibcSocket::new(&sys)
            .open_local_dgram()?
            .get_lladdr(ifreq::as_mut_ptr(&mut ifreq))?;
        // Then
        assert_eq!(ifreq::get_lladdr(&ifreq), expected_lladdr);
        Ok(())
    }

    // #[test]
    // fn test_local_dgram_socket_get_lladdr_err() -> Result<()> {
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
    fn test_local_dgram_socket_set_lladdr() -> Result<()> {
        // Given
        let ifname: IfName = "en".try_into()?;
        let lladdr: LLAddr = "00:11:22:33:44:55".parse()?;
        let sys = MockSys::default();
        let mut ifreq = ifreq::new();
        ifreq::set_name(&mut ifreq, &ifname);
        ifreq::set_lladdr(&mut ifreq, &lladdr);
        // When
        LibcSocket::new(&sys)
            .open_local_dgram()?
            .set_lladdr(ifreq::as_mut_ptr(&mut ifreq))?;
        // Then
        assert!(sys.has_nic(&ifname, &lladdr));
        Ok(())
    }

    // #[test]
    // fn test_local_dgram_socket_set_lladdr_err() -> Result<()> {
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
    use crate::{
        macos::ifreq::{self},
        IfName, LinkLevelAddress, Result,
    };

    use super::{OpenSocket, Socket};
    use std::{cell::RefCell, collections::HashMap, rc::Rc};

    type KeyValue = RefCell<HashMap<IfName, LinkLevelAddress>>;

    #[derive(Clone, Debug, Default)]
    pub(crate) struct MockSocket {
        kv: Rc<KeyValue>,
    }

    impl MockSocket {
        pub(crate) fn with_nic(self, ifname: IfName, lladdr: LinkLevelAddress) -> Self {
            self.set_nic(ifname, lladdr);
            self
        }

        pub(crate) fn set_nic(&self, ifname: IfName, lladdr: LinkLevelAddress) {
            self.kv.borrow_mut().insert(ifname, lladdr);
        }

        pub(crate) fn has_nic(&self, ifname: &IfName, expected_lladdr: &LinkLevelAddress) -> bool {
            match self.kv.borrow().get(ifname) {
                Some(lladdr) => lladdr == expected_lladdr,
                None => false,
            }
        }
    }

    impl Socket for MockSocket {
        fn open_local_dgram(&self) -> Result<Box<dyn OpenSocket + '_>> {
            eprintln!("MockSocket.open_local_dgram()");
            Ok(Box::new(MockOpenSocket { kv: &self.kv }))
        }
    }

    pub(crate) struct MockOpenSocket<'a> {
        kv: &'a Rc<KeyValue>,
    }

    impl<'a> OpenSocket for MockOpenSocket<'a> {
        fn get_lladdr(&self, arg: *mut libc::c_void) -> Result<()> {
            let ifreq = ifreq::from_mut_ptr(arg);
            let ifname: IfName = ifreq::get_name(ifreq);

            if let Some(lladdr) = self.kv.borrow().get(&ifname) {
                eprintln!("MockOpenSocket.get_lladdr({ifname}) -> {lladdr})");
                ifreq::set_lladdr(ifreq, lladdr)
            };
            Ok(())
        }

        fn set_lladdr(&self, arg: *mut libc::c_void) -> Result<()> {
            let ifreq = ifreq::from_mut_ptr(arg);
            let ifname = ifreq::get_name(ifreq);
            let lladdr = ifreq::get_lladdr(ifreq);

            eprintln!("MockOpenSocket.set_lladdr({ifname}, {lladdr})");
            self.kv.borrow_mut().insert(ifname, lladdr);

            Ok(())
        }
    }
}
