use std::io;
use std::mem::{size_of, size_of_val, MaybeUninit};
use std::os::fd::AsRawFd;
use std::os::unix::io::RawFd;
use std::os::unix::net::UnixStream;

use std::ffi::{c_int, c_uint, c_void};

use libc::{
    cmsghdr, msghdr, recvmsg, sendmsg, CMSG_DATA, CMSG_LEN, CMSG_NXTHDR, CMSG_SPACE, SCM_RIGHTS,
    SOL_SOCKET,
};

pub trait ScmRightsSocket {
    fn recvmsg_fds(&mut self, buf: &mut [u8], fds: &mut [RawFd]) -> io::Result<(usize, usize)>;
    fn sendmsg_fds(&mut self, buf: &[u8], fds: &[RawFd]) -> io::Result<usize>;
}

#[cfg(not(target_os = "linux"))]
const fn cmsg_len(len: c_uint) -> c_uint {
    // SAFETY: CMSG_LEN is a simple const-fn doing integer maths
    unsafe { CMSG_LEN(len) }
}
#[cfg(target_os = "linux")]
const fn cmsg_len(len: c_uint) -> usize {
    // SAFETY: CMSG_LEN is a simple const-fn doing integer maths
    unsafe { CMSG_LEN(len) as usize }
}

const fn cmsg_space(len: c_uint) -> c_uint {
    // SAFETY: CMSG_SPACE is a simple const-fn doing integer maths
    unsafe { CMSG_SPACE(len) }
}

fn cmsg_nexthdr(hdr: &msghdr, cmsg: *const cmsghdr) -> *const cmsghdr {
    unsafe { CMSG_NXTHDR(hdr as *const msghdr, cmsg) }
}

struct CmsgBuf(Vec<c_int>);
impl CmsgBuf {
    fn for_send(fds: &[RawFd]) -> Self {
        let slots = fds.len();
        let mut buf = Self::buf_alloc(slots);

        unsafe {
            let dataptr = CMSG_DATA(buf.as_mut_ptr() as *mut cmsghdr) as *mut c_int;
            dataptr.copy_from_nonoverlapping(fds.as_ptr(), fds.len());
        }

        let hdr = cmsghdr {
            cmsg_len: cmsg_len(size_of_val(fds) as u32),
            cmsg_level: SOL_SOCKET,
            cmsg_type: SCM_RIGHTS,
        };
        unsafe {
            let hdrptr = buf.as_mut_ptr() as *mut cmsghdr;
            hdrptr.copy_from_nonoverlapping(&hdr, 1);
        }

        Self(buf)
    }
    fn for_recv(slots: usize) -> Self {
        Self(Self::buf_alloc(slots))
    }
    fn buf_alloc(slots: usize) -> Vec<c_int> {
        let fd_sz = slots * size_of::<c_int>();
        let buf_sz = cmsg_space(fd_sz as u32) as usize;
        let buf_len = buf_sz.div_ceil(size_of::<c_int>());
        let mut buf = Vec::with_capacity(buf_len);
        buf.resize_with(buf_len, Default::default);
        buf
    }

    /// Get the buffer (msg_control) and length (msg_controllen) to use with
    /// [sendmsg()]/[recvmsg()]
    #[cfg(not(target_os = "linux"))]
    unsafe fn get(&mut self) -> (*mut c_void, c_uint) {
        (
            self.0.as_mut_ptr() as *mut c_void,
            (self.0.len() * size_of::<c_int>()) as c_uint,
        )
    }
    #[cfg(target_os = "linux")]
    unsafe fn get(&mut self) -> (*mut c_void, libc::size_t) {
        (
            self.0.as_mut_ptr() as *mut c_void,
            (self.0.len() * size_of::<c_int>()),
        )
    }

    /// Parse the result of a [recvmsg()] call
    ///
    /// # Panics
    /// - If `msghdr.msg_control` does not point to the buffer contained in this [CmsgBuf].
    /// - If `msghdr.msg_controllen` exceeds the buffer length contained in this [CmsgBuf].
    unsafe fn parse_from_recv(&mut self, msghdr: &msghdr, mut fds: &mut [RawFd]) -> usize {
        assert_eq!(msghdr.msg_control, self.get().0);
        assert!(msghdr.msg_controllen <= self.get().1);

        let mut cmsg = msghdr.msg_control as *const cmsghdr;
        let mut valid_fds = 0;
        while !cmsg.is_null() {
            let hdr = unsafe {
                let mut hdr_uninit = MaybeUninit::uninit();
                cmsg.copy_to_nonoverlapping(hdr_uninit.as_mut_ptr(), 1);
                hdr_uninit.assume_init()
            };
            if hdr.cmsg_level == SOL_SOCKET && hdr.cmsg_type == SCM_RIGHTS {
                #[cfg(not(target_os = "linux"))]
                let count = (hdr.cmsg_len - cmsg_len(0)) / size_of::<c_int>() as u32;
                #[cfg(target_os = "linux")]
                let count = (hdr.cmsg_len - cmsg_len(0)) / size_of::<c_int>();

                let to_copy = usize::min(count as usize, fds.len());
                if to_copy == 0 {
                    break;
                }
                unsafe {
                    CMSG_DATA(cmsg).copy_to_nonoverlapping(
                        fds.as_mut_ptr() as *mut u8,
                        size_of::<RawFd>() * to_copy,
                    );
                }
                valid_fds += to_copy;
                fds = &mut fds[to_copy..];
            }
            cmsg = cmsg_nexthdr(msghdr, cmsg);
        }

        valid_fds
    }
}

impl ScmRightsSocket for UnixStream {
    fn recvmsg_fds(&mut self, buf: &mut [u8], fds: &mut [RawFd]) -> io::Result<(usize, usize)> {
        let fd_space = size_of_val(fds);
        let cmsg_buf_sz = unsafe { libc::CMSG_SPACE(fd_space as u32) };

        // Allocate cmsg buffer as fd-sized array to meet alignment guarantees for later
        let cmsg_buf_elems = cmsg_buf_sz.div_ceil(size_of::<c_int>() as u32) as usize;
        let mut cmsg_buf: Vec<c_int> = Vec::with_capacity(cmsg_buf_elems);
        cmsg_buf.resize_with(cmsg_buf_elems, Default::default);

        let mut cmsgs = CmsgBuf::for_recv(fds.len());
        let (msg_control, msg_controllen) = unsafe { cmsgs.get() };
        let mut iovec = libc::iovec {
            iov_base: buf.as_mut_ptr() as *mut c_void,
            iov_len: buf.len(),
        };
        let mut hdr = msghdr {
            msg_name: std::ptr::null_mut(),
            msg_namelen: 0,
            msg_iov: &mut iovec,
            msg_iovlen: 1,
            msg_control,
            msg_controllen,
            msg_flags: 0,
        };

        let sockfd = self.as_raw_fd();
        let nread = unsafe { recvmsg(sockfd, &mut hdr, 0) };
        if nread == -1 {
            Err(std::io::Error::last_os_error())
        } else {
            let valid = unsafe { cmsgs.parse_from_recv(&hdr, fds) };
            Ok((nread as usize, valid))
        }
    }

    fn sendmsg_fds(&mut self, buf: &[u8], fds: &[RawFd]) -> io::Result<usize> {
        let mut cmsgs = CmsgBuf::for_send(fds);

        let (msg_control, msg_controllen) = unsafe { cmsgs.get() };
        let mut iovec = libc::iovec {
            iov_base: buf.as_ptr() as *mut c_void,
            iov_len: buf.len(),
        };
        let hdr = msghdr {
            msg_name: std::ptr::null_mut(),
            msg_namelen: 0,
            msg_iov: &mut iovec,
            msg_iovlen: 1,
            msg_control,
            msg_controllen,
            msg_flags: 0,
        };

        let sockfd = self.as_raw_fd();
        let nread = unsafe { sendmsg(sockfd, &hdr, 0) };
        if nread == -1 {
            Err(std::io::Error::last_os_error())
        } else {
            Ok(nread as usize)
        }
    }
}
