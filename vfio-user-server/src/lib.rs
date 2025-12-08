// Copyright © 2021 Intel Corporation
//
// SPDX-License-Identifier: Apache-2.0
//

use bytemuck::{bytes_of, bytes_of_mut};
use libc::EINVAL;
use log::*;
use std::ffi::CString;
use std::fs::File;
use std::io::{IoSlice, Read, Write};
use std::mem::size_of;
use std::num::Wrapping;
use std::os::unix::{
    io::{FromRawFd, RawFd},
    net::{UnixListener, UnixStream},
};
use std::path::Path;
use thiserror::Error;
use vfio_user_proto::{vfio_sys::*, *};

mod scm_sock;
use scm_sock::ScmRightsSocket;

const SERVER_DEFAULT_CAPS: Capabilities = Capabilities {
    max_msg_fds: Some(1),
    max_data_xfer_size: Some(1024 * 1024),
    migration: None,
};

pub struct Client {
    stream: UnixStream,
    next_message_id: Wrapping<u16>,
    num_irqs: u32,
    resettable: bool,
    regions: Vec<Region>,
}

#[derive(Debug)]
pub struct Region {
    pub flags: u32,
    pub index: u32,
    pub size: u64,
    pub file_offset: Option<(File, u64)>,
    pub sparse_areas: Vec<vfio_region_sparse_mmap_area>,
}

#[derive(Debug)]
pub struct IrqInfo {
    pub index: u32,
    pub flags: u32,
    pub count: u32,
}

#[derive(Error, Debug)]
pub enum Error {
    #[error("Error connecting: {0}")]
    Connect(#[source] std::io::Error),
    #[error("Error serializing capabilities: {0}")]
    SerializeCapabilites(#[source] serde_json::Error),
    #[error("Error deserializing capabilities: {0}")]
    DeserializeCapabilites(#[source] serde_json::Error),
    #[error("Error writing to stream: {0}")]
    StreamWrite(#[source] std::io::Error),
    #[error("Error reading from stream: {0}")]
    StreamRead(#[source] std::io::Error),
    #[error("Error shutting down stream: {0}")]
    StreamShutdown(#[source] std::io::Error),
    #[error("Not a PCI device")]
    NotPciDevice,
    #[error("Error binding to socket: {0}")]
    SocketBind(#[source] std::io::Error),
    #[error("Error accepting connection: {0}")]
    SocketAccept(#[source] std::io::Error),
    #[error("Unsupported command: {0:?}")]
    UnsupportedCommand(Command),
    #[error("Unrecognized command: {0:#}")]
    UnrecognizedCommand(u16),
    #[error("Unsupported feature")]
    UnsupportedFeature,
    #[error("Error from backend: {0:?}")]
    Backend(#[source] std::io::Error),
    #[error("Invalid input")]
    InvalidInput,
}

impl Client {
    pub fn new(path: &Path) -> Result<Client, Error> {
        let stream = UnixStream::connect(path).map_err(Error::Connect)?;

        let mut client = Client {
            next_message_id: Wrapping(0),
            stream,
            num_irqs: 0,
            resettable: false,
            regions: Vec::new(),
        };

        client.negotiate_version()?;

        client.regions = client.get_regions()?;

        Ok(client)
    }

    fn negotiate_version(&mut self) -> Result<(), Error> {
        let caps = VersionData {
            capabilities: SERVER_DEFAULT_CAPS,
        };

        let version_data = serde_json::to_string(&caps).map_err(Error::SerializeCapabilites)?;

        let version = Version {
            header: Header {
                message_id: self.next_message_id.0,
                command: Command::Version.into(),
                flags: HeaderFlags::command().into(),
                message_size: (size_of::<Version>() + version_data.len() + 1) as u32,
                ..Default::default()
            },
            payload: VersionPayload { major: 0, minor: 1 },
        };
        debug!("Command: {version:?}");

        let version_data = CString::new(version_data.as_bytes()).unwrap();
        let bufs = vec![
            IoSlice::new(bytes_of(&version)),
            IoSlice::new(version_data.as_bytes_with_nul()),
        ];

        // TODO: Use write_all_vectored() when ready
        let _ = self
            .stream
            .write_vectored(&bufs)
            .map_err(Error::StreamWrite)?;

        debug!(
            "Sent client version information: major = {} minor = {} capabilities = {:?}",
            version.payload.major, version.payload.minor, &caps.capabilities
        );

        self.next_message_id += Wrapping(1);

        let mut server_version: Version = Version::default();
        self.stream
            .read_exact(bytes_of_mut(&mut server_version))
            .map_err(Error::StreamRead)?;

        debug!("Reply: {server_version:?}");

        let mut server_version_data =
            vec![0; server_version.header.message_size as usize - size_of::<Version>()];
        self.stream
            .read_exact(&mut server_version_data)
            .map_err(Error::StreamRead)?;

        let server_caps: VersionData =
            serde_json::from_slice(&server_version_data[0..server_version_data.len() - 1])
                .map_err(Error::DeserializeCapabilites)?;

        debug!(
            "Received server version information: major = {} minor = {} capabilities = {:?}",
            server_version.payload.major, server_version.payload.minor, &server_caps.capabilities
        );

        Ok(())
    }

    pub fn dma_map(
        &mut self,
        offset: u64,
        address: u64,
        size: u64,
        fd: RawFd,
    ) -> Result<(), Error> {
        let dma_map = DmaMap {
            header: Header {
                message_id: self.next_message_id.0,
                command: Command::DmaMap.into(),
                flags: HeaderFlags::command().into(),
                message_size: size_of::<DmaMap>() as u32,
                ..Default::default()
            },
            payload: DmaMapPayload {
                argsz: size_of::<DmaMapPayload>() as u32,
                flags: DmaMapFlags::READ_WRITE.bits(),
                offset,
                address,
                size,
            },
        };
        debug!("Command: {dma_map:?}");
        self.next_message_id += Wrapping(1);
        self.stream
            .sendmsg_fds(bytes_of(&dma_map), &[fd])
            .map_err(Error::StreamWrite)?;

        let mut reply = Header::default();
        self.stream
            .read_exact(bytes_of_mut(&mut reply))
            .map_err(Error::StreamRead)?;
        debug!("Reply: {reply:?}");

        Ok(())
    }

    pub fn dma_unmap(&mut self, address: u64, size: u64) -> Result<(), Error> {
        let dma_unmap = DmaUnmap {
            header: Header {
                message_id: self.next_message_id.0,
                command: Command::DmaUnmap.into(),
                flags: HeaderFlags::command().into(),
                message_size: size_of::<DmaUnmap>() as u32,
                ..Default::default()
            },
            payload: DmaUnmapPayload {
                argsz: size_of::<DmaUnmapPayload>() as u32,
                flags: 0,
                address,
                size,
            },
        };
        debug!("Command: {dma_unmap:?}");
        self.next_message_id += Wrapping(1);
        self.stream
            .write_all(bytes_of(&dma_unmap))
            .map_err(Error::StreamWrite)?;

        let mut reply = DmaUnmap::default();
        self.stream
            .read_exact(bytes_of_mut(&mut reply))
            .map_err(Error::StreamRead)?;
        debug!("Reply: {reply:?}");

        Ok(())
    }

    pub fn reset(&mut self) -> Result<(), Error> {
        let reset = DeviceReset {
            header: Header {
                message_id: self.next_message_id.0,
                command: Command::DeviceReset.into(),
                flags: HeaderFlags::command().into(),
                message_size: size_of::<DeviceReset>() as u32,
                ..Default::default()
            },
        };
        debug!("Command: {reset:?}");
        self.next_message_id += Wrapping(1);
        self.stream
            .write_all(bytes_of(&reset))
            .map_err(Error::StreamWrite)?;

        let mut reply = Header::default();
        self.stream
            .read_exact(bytes_of_mut(&mut reply))
            .map_err(Error::StreamRead)?;
        debug!("Reply: {reply:?}");

        Ok(())
    }

    fn get_regions(&mut self) -> Result<Vec<Region>, Error> {
        let get_info = DeviceGetInfo {
            header: Header {
                message_id: self.next_message_id.0,
                command: Command::DeviceGetInfo.into(),
                flags: HeaderFlags::command().into(),
                message_size: size_of::<DeviceGetInfo>() as u32,
                ..Default::default()
            },
            payload: DeviceGetInfoPayload {
                argsz: size_of::<DeviceGetInfoPayload>() as u32,
                ..Default::default()
            },
        };
        debug!("Command: {get_info:?}");
        self.next_message_id += Wrapping(1);

        self.stream
            .write_all(bytes_of(&get_info))
            .map_err(Error::StreamWrite)?;

        let mut replymsg = DeviceGetInfo::default();
        self.stream
            .read_exact(bytes_of_mut(&mut replymsg))
            .map_err(Error::StreamRead)?;
        let reply = &replymsg.payload;
        debug!("Reply: {reply:?}");
        self.num_irqs = reply.num_irqs;

        if reply.flags & VFIO_DEVICE_FLAGS_PCI != VFIO_DEVICE_FLAGS_PCI {
            return Err(Error::NotPciDevice);
        }

        self.resettable = reply.flags & VFIO_DEVICE_FLAGS_RESET != VFIO_DEVICE_FLAGS_RESET;

        let num_regions = reply.num_regions;
        let mut regions = Vec::new();
        for index in 0..num_regions {
            let (region_info, fd, sparse_areas) = self.get_region_info(index)?;
            regions.push(Region {
                flags: region_info.flags,
                index: region_info.index,
                size: region_info.size,
                file_offset: fd.map(|fd| (fd, region_info.offset)),
                sparse_areas,
            });
        }

        Ok(regions)
    }

    fn get_region_info(
        &mut self,
        index: u32,
    ) -> Result<
        (
            vfio_region_info,
            Option<File>,
            Vec<vfio_region_sparse_mmap_area>,
        ),
        Error,
    > {
        // Retrieve the region info without capability
        let mut get_region_info = DeviceGetRegionInfo {
            header: Header {
                message_id: self.next_message_id.0,
                command: Command::DeviceGetRegionInfo.into(),
                flags: HeaderFlags::command().into(),
                message_size: std::mem::size_of::<DeviceGetRegionInfo>() as u32,
                ..Default::default()
            },
            payload: vfio_region_info {
                argsz: size_of::<vfio_region_info>() as u32,
                index,
                ..Default::default()
            },
        };
        debug!("Command: {get_region_info:?}");
        self.next_message_id += Wrapping(1);

        self.stream
            .write_all(bytes_of(&get_region_info))
            .map_err(Error::StreamWrite)?;

        let mut reply = DeviceGetRegionInfo::default();
        let fd = -1;
        let (_bytes_recvd, _fds_recvd) = self
            .stream
            .recvmsg_fds(bytes_of_mut(&mut reply), &mut [fd])
            .map_err(Error::StreamRead)?;
        debug!("Reply: {reply:?}");

        // Retrieve the region info again with capabilities if needed
        let sparse_areas = if reply.payload.argsz > std::mem::size_of::<vfio_region_info>() as u32 {
            get_region_info.payload.argsz = reply.payload.argsz;
            debug!("Command: {get_region_info:?}");
            self.next_message_id += Wrapping(1);

            self.stream
                .write_all(bytes_of(&get_region_info))
                .map_err(Error::StreamWrite)?;

            let mut reply = DeviceGetRegionInfo::default();
            let (_bytes_recvd, _fds_recvd) = self
                .stream
                .recvmsg_fds(bytes_of_mut(&mut reply), &mut [fd])
                .map_err(Error::StreamRead)?;
            debug!("Reply: {reply:?}");

            let cap_size = reply.payload.argsz - std::mem::size_of::<vfio_region_info>() as u32;
            assert_eq!(
                cap_size,
                reply.header.message_size - size_of::<DeviceGetRegionInfo>() as u32
            );
            let mut cap_data = vec![0u8; cap_size as usize];
            self.stream
                .read_exact(&mut cap_data)
                .map_err(Error::StreamRead)?;

            Self::parse_region_caps(&cap_data, &reply.payload)?
        } else {
            vec![]
        };

        let fp = if fd != -1 {
            unsafe { Some(File::from_raw_fd(fd)) }
        } else {
            None
        };
        Ok((reply.payload, fp, sparse_areas))
    }

    fn parse_region_caps(
        cap_data: &[u8],
        region_info: &vfio_region_info,
    ) -> Result<Vec<vfio_region_sparse_mmap_area>, Error> {
        let mut sparse_areas: Vec<vfio_region_sparse_mmap_area> = Vec::new();

        let cap_size = cap_data.len() as u32;
        let cap_header_size = size_of::<vfio_info_cap_header>() as u32;
        let mmap_cap_size = size_of::<vfio_region_info_cap_sparse_mmap>() as u32;
        let mmap_area_size = size_of::<vfio_region_sparse_mmap_area>() as u32;

        let cap_data_ptr = cap_data.as_ptr();
        let mut region_info_offset = region_info.cap_offset;
        while region_info_offset != 0 {
            // calculate the offset from the begining of the cap_data based on the offset
            // that is relative to the begining of the VFIO region info structure
            let cap_offset = region_info_offset - size_of::<vfio_region_info>() as u32;
            if cap_offset + cap_header_size > cap_size {
                warn!(
                    "Unexpected end of cap data: 'cap_offset + cap_header_size > cap_size' \
                cap_offset = {cap_offset}, cap_header_size = {cap_header_size}, cap_size = {cap_size}"
                );
                break;
            }

            // SAFETY: `cap_data_ptr` is valid and the `cap_offset` is checked above
            let cap_ptr = unsafe { cap_data_ptr.offset(cap_offset as isize) };
            // SAFETY: `cap_ptr` is valid
            let cap_header = unsafe { &*(cap_ptr as *const vfio_info_cap_header) };
            match cap_header.id as u32 {
                VFIO_REGION_INFO_CAP_SPARSE_MMAP => {
                    if cap_offset + mmap_cap_size > cap_size {
                        warn!(
                            "Unexpected end of cap data: 'cap_offset + mmap_cap_size > cap_size' \
                        cap_offset = {cap_offset}, mmap_cap_size = {mmap_cap_size}, cap_size = {cap_size}"
                        );
                        break;
                    }
                    // SAFETY: `cap_ptr` is valid and its size is also checked above
                    let sparse_mmap = unsafe {
                        &*(cap_ptr as *mut u8 as *const vfio_region_info_cap_sparse_mmap)
                    };

                    let area_num = sparse_mmap.nr_areas;
                    if cap_offset + mmap_cap_size + area_num * mmap_area_size > cap_size {
                        warn!(
                            "Unexpected end of cap data: 'cap_offset + mmap_cap_size + area_num * mmap_area_size > cap_size' \
                        cap_offset = {cap_offset}, mmap_cap_size = {mmap_area_size}, area_num = {area_num}, mmap_area_size = {mmap_area_size}, cap_size = {cap_size}"
                        );
                        break;
                    }
                    let areas =
                        // SAFETY: `sparse_mmap` is valid and its size is also checked above
                        unsafe { sparse_mmap.areas.as_slice(sparse_mmap.nr_areas as usize) };
                    for area in areas.iter() {
                        sparse_areas.push(*area);
                    }
                }
                _ => {
                    warn!(
                        "Ignoring unsupported vfio region capability (id = '{}')",
                        cap_header.id
                    );
                }
            }
            region_info_offset = cap_header.next;
        }

        Ok(sparse_areas)
    }

    pub fn region_read(&mut self, region: u32, offset: u64, data: &mut [u8]) -> Result<(), Error> {
        let region_read = RegionAccess {
            header: Header {
                message_id: self.next_message_id.0,
                command: Command::RegionRead.into(),
                flags: HeaderFlags::command().into(),
                message_size: size_of::<RegionAccess>() as u32,
                ..Default::default()
            },
            payload: RegionAccessPayload {
                offset,
                count: data.len() as u32,
                region,
            },
        };
        debug!("Command: {region_read:?}");
        self.next_message_id += Wrapping(1);
        self.stream
            .write_all(bytes_of(&region_read))
            .map_err(Error::StreamWrite)?;

        let mut reply = RegionAccess::default();
        self.stream
            .read_exact(bytes_of_mut(&mut reply))
            .map_err(Error::StreamRead)?;
        debug!("Reply: {reply:?}");
        self.stream.read_exact(data).map_err(Error::StreamRead)?;
        Ok(())
    }

    pub fn region_write(&mut self, region: u32, offset: u64, data: &[u8]) -> Result<(), Error> {
        let region_write = RegionAccess {
            header: Header {
                message_id: self.next_message_id.0,
                command: Command::RegionWrite.into(),
                flags: HeaderFlags::command().into(),
                message_size: (size_of::<RegionAccess>() + data.len()) as u32,
                ..Default::default()
            },
            payload: RegionAccessPayload {
                offset,
                count: data.len() as u32,
                region,
            },
        };
        debug!("Command: {region_write:?}");
        self.next_message_id += Wrapping(1);

        let bufs = vec![IoSlice::new(bytes_of(&region_write)), IoSlice::new(data)];

        // TODO: Use write_all_vectored() when ready
        let _ = self
            .stream
            .write_vectored(&bufs)
            .map_err(Error::StreamWrite)?;

        let mut reply = RegionAccess::default();
        self.stream
            .read_exact(bytes_of_mut(&mut reply))
            .map_err(Error::StreamRead)?;
        debug!("Reply: {reply:?}");
        Ok(())
    }

    pub fn get_irq_info(&mut self, index: u32) -> Result<IrqInfo, Error> {
        let get_irq_info = GetIrqInfo {
            header: Header {
                message_id: self.next_message_id.0,
                command: Command::GetIrqInfo.into(),
                flags: HeaderFlags::command().into(),
                message_size: size_of::<GetIrqInfo>() as u32,
                ..Default::default()
            },
            payload: GetIrqInfoPayload {
                argsz: size_of::<GetIrqInfoPayload>() as u32,
                flags: 0,
                index,
                count: 0,
            },
        };
        debug!("Command: {get_irq_info:?}");
        self.next_message_id += Wrapping(1);

        self.stream
            .write_all(bytes_of(&get_irq_info))
            .map_err(Error::StreamWrite)?;

        let mut replymsg = GetIrqInfo::default();
        self.stream
            .read_exact(bytes_of_mut(&mut replymsg))
            .map_err(Error::StreamRead)?;
        let reply = &replymsg.payload;
        debug!("Reply: {reply:?}");

        Ok(IrqInfo {
            index: reply.index,
            flags: reply.flags,
            count: reply.count,
        })
    }

    pub fn set_irqs(
        &mut self,
        index: u32,
        flags: u32,
        start: u32,
        count: u32,
        fds: &[RawFd],
    ) -> Result<(), Error> {
        let set_irqs = SetIrqs {
            header: Header {
                message_id: self.next_message_id.0,
                command: Command::SetIrqs.into(),
                flags: HeaderFlags::command().into(),
                message_size: size_of::<SetIrqs>() as u32,
                ..Default::default()
            },
            payload: SetIrqsPayload {
                argsz: size_of::<SetIrqsPayload>() as u32,
                flags,
                start,
                index,
                count,
            },
        };
        debug!("Command: {set_irqs:?}");
        self.next_message_id += Wrapping(1);

        self.stream
            .sendmsg_fds(bytes_of(&set_irqs), fds)
            .map_err(Error::StreamWrite)?;

        let mut reply = Header::default();
        self.stream
            .read_exact(bytes_of_mut(&mut reply))
            .map_err(Error::StreamRead)?;
        debug!("Reply: {reply:?}");

        Ok(())
    }

    pub fn region(&self, region_index: u32) -> Option<&Region> {
        self.regions
            .iter()
            .find(|&region| region.index == region_index)
    }

    pub fn resettable(&self) -> bool {
        self.resettable
    }

    pub fn shutdown(&self) -> Result<(), Error> {
        self.stream
            .shutdown(std::net::Shutdown::Both)
            .map_err(Error::StreamShutdown)
    }
}

pub trait ServerBackend {
    fn region_read(
        &mut self,
        _region: u32,
        _offset: u64,
        _data: &mut [u8],
    ) -> Result<(), std::io::Error>;
    fn region_write(
        &mut self,
        _region: u32,
        _offset: u64,
        _data: &[u8],
    ) -> Result<(), std::io::Error>;
    fn dma_map(
        &mut self,
        _flags: DmaMapFlags,
        _offset: u64,
        _address: u64,
        _size: u64,
        _fd: Option<File>,
    ) -> Result<(), std::io::Error>;
    fn dma_unmap(
        &mut self,
        _flags: DmaUnmapFlags,
        _address: u64,
        _size: u64,
    ) -> Result<(), std::io::Error>;
    fn reset(&mut self) -> Result<(), std::io::Error>;
    fn set_irqs(
        &mut self,
        _index: u32,
        _flags: u32,
        _start: u32,
        _count: u32,
        _fds: Vec<File>,
    ) -> Result<(), std::io::Error>;
}

pub struct Server {
    listener: UnixListener,
    resettable: bool,
    irqs: Vec<IrqInfo>,
    regions: Vec<vfio_region_info>,
}

impl Server {
    pub fn new(
        path: &Path,
        resettable: bool,
        irqs: Vec<IrqInfo>,
        regions: Vec<vfio_region_info>,
    ) -> Result<Server, Error> {
        let listener = UnixListener::bind(path).map_err(Error::SocketBind)?;

        Ok(Server {
            listener,
            resettable,
            irqs,
            regions,
        })
    }

    fn handle_command(
        &self,
        backend: &mut dyn ServerBackend,
        stream: &mut UnixStream,
        header: Header,
        fds: Vec<File>,
    ) -> Result<(), Error> {
        let parsed_cmd = Command::try_from(header.command).map_err(Error::UnrecognizedCommand)?;
        match parsed_cmd {
            Command::Unknown
            | Command::GetRegionIoFds
            | Command::DmaRead
            | Command::DmaWrite
            | Command::UserDirtyPages => {
                return Err(Error::UnsupportedCommand(parsed_cmd));
            }
            Command::Version => {
                // TODO: Make version/capabilities configurable
                let mut client_version = VersionPayload::default();
                stream
                    .read_exact(bytes_of_mut(&mut client_version))
                    .map_err(Error::StreamRead)?;

                let mut raw_version_data =
                    vec![0; header.message_size as usize - size_of::<Version>()];
                stream
                    .read_exact(&mut raw_version_data)
                    .map_err(Error::StreamRead)?;
                let client_version_data = CString::from_vec_with_nul(raw_version_data)
                    .unwrap()
                    .to_string_lossy()
                    .into_owned();
                let client_capabilities: VersionData = serde_json::from_str(&client_version_data)
                    .map_err(Error::DeserializeCapabilites)?;

                info!(
                    "Received client version: major = {} minor = {} capabilities = {:?}",
                    client_version.major, client_version.minor, client_capabilities.capabilities,
                );

                let server_capabilities = VersionData::default();
                let server_version_data = serde_json::to_string(&server_capabilities)
                    .map_err(Error::SerializeCapabilites)?;
                let server_version = Version {
                    header: Header {
                        message_id: header.message_id,
                        command: Command::Version.into(),
                        flags: HeaderFlags::reply().into(),
                        message_size: (size_of::<Version>() + server_version_data.len() + 1) as u32,
                        ..Default::default()
                    },
                    payload: VersionPayload { major: 0, minor: 0 },
                };

                let server_version_data = CString::new(server_version_data.as_bytes()).unwrap();

                let bufs = vec![
                    IoSlice::new(bytes_of(&server_version)),
                    IoSlice::new(server_version_data.as_bytes_with_nul()),
                ];

                // TODO: Use write_all_vectored() when ready
                let _ = stream.write_vectored(&bufs).map_err(Error::StreamWrite)?;

                info!(
                    "Sent server version: major = {} minor = {} capabilities = {:?}",
                    server_version.payload.major,
                    server_version.payload.minor,
                    server_capabilities.capabilities
                );
            }
            Command::DmaMap => {
                let mut payload = DmaMapPayload::default();
                stream
                    .read_exact(bytes_of_mut(&mut payload))
                    .map_err(Error::StreamRead)?;

                let mut fds = fds;

                // The specification demands that the caller passes 0
                // or 1 file descriptor.
                if fds.len() > 1 {
                    return Err(Error::InvalidInput);
                }

                backend
                    .dma_map(
                        DmaMapFlags::from_bits_truncate(payload.flags),
                        payload.offset,
                        payload.address,
                        payload.size,
                        fds.pop(),
                    )
                    .map_err(Error::Backend)?;

                let reply = Header {
                    message_id: header.message_id,
                    command: Command::DmaMap.into(),
                    flags: HeaderFlags::reply().into(),
                    message_size: size_of::<Header>() as u32,
                    ..Default::default()
                };
                stream
                    .write_all(bytes_of(&reply))
                    .map_err(Error::StreamWrite)?;
            }
            Command::DmaUnmap => {
                let mut payload = DmaUnmapPayload::default();
                stream
                    .read_exact(bytes_of_mut(&mut payload))
                    .map_err(Error::StreamRead)?;

                backend
                    .dma_unmap(
                        DmaUnmapFlags::from_bits_truncate(payload.flags),
                        payload.address,
                        payload.size,
                    )
                    .map_err(Error::Backend)?;

                let reply = DmaUnmap {
                    header: Header {
                        message_id: header.message_id,
                        command: Command::DmaUnmap.into(),
                        flags: HeaderFlags::reply().into(),
                        message_size: size_of::<DmaUnmap>() as u32,
                        ..Default::default()
                    },
                    payload,
                };
                stream
                    .write_all(bytes_of(&reply))
                    .map_err(Error::StreamWrite)?;
            }
            Command::DeviceGetInfo => {
                let mut payload = DeviceGetInfoPayload::default();
                stream
                    .read_exact(bytes_of_mut(&mut payload))
                    .map_err(Error::StreamRead)?;

                let reply = DeviceGetInfo {
                    header: Header {
                        message_id: header.message_id,
                        command: Command::DeviceGetInfo.into(),
                        flags: HeaderFlags::reply().into(),
                        message_size: size_of::<DeviceGetInfo>() as u32,
                        ..Default::default()
                    },
                    payload: DeviceGetInfoPayload {
                        argsz: size_of::<DeviceGetInfoPayload>() as u32,
                        // TODO: Consider non-PCI devices
                        flags: VFIO_DEVICE_FLAGS_PCI
                            | if self.resettable {
                                VFIO_DEVICE_FLAGS_RESET
                            } else {
                                0
                            },
                        num_regions: self.regions.len() as u32,
                        num_irqs: self.irqs.len() as u32,
                    },
                };
                stream
                    .write_all(bytes_of(&reply))
                    .map_err(Error::StreamWrite)?;
            }
            Command::DeviceGetRegionInfo => {
                let mut payload = vfio_region_info::default();
                stream
                    .read_exact(bytes_of_mut(&mut payload))
                    .map_err(Error::StreamRead)?;

                if payload.index as usize >= self.regions.len() {
                    return Err(Error::InvalidInput);
                }

                // TODO: Need to handle region capabilities e.g. sparse regions
                let reply = DeviceGetRegionInfo {
                    header: Header {
                        message_id: header.message_id,
                        command: Command::DeviceGetRegionInfo.into(),
                        flags: HeaderFlags::reply().into(),
                        message_size: size_of::<DeviceGetRegionInfo>() as u32,
                        ..Default::default()
                    },
                    payload: self.regions[payload.index as usize],
                };
                stream
                    .write_all(bytes_of(&reply))
                    .map_err(Error::StreamWrite)?;
            }
            Command::GetIrqInfo => {
                let mut payload = GetIrqInfoPayload::default();
                stream
                    .read_exact(bytes_of_mut(&mut payload))
                    .map_err(Error::StreamRead)?;

                if payload.index as usize >= self.irqs.len() {
                    return Err(Error::InvalidInput);
                }

                let irq = &self.irqs[payload.index as usize];

                let reply = GetIrqInfo {
                    header: Header {
                        message_id: header.message_id,
                        command: Command::GetIrqInfo.into(),
                        flags: HeaderFlags::reply().into(),
                        message_size: size_of::<GetIrqInfo>() as u32,
                        ..Default::default()
                    },
                    payload: GetIrqInfoPayload {
                        argsz: size_of::<GetIrqInfoPayload>() as u32,
                        index: irq.index,
                        flags: irq.flags,
                        count: irq.count,
                    },
                };
                stream
                    .write_all(bytes_of(&reply))
                    .map_err(Error::StreamWrite)?;
            }
            Command::SetIrqs => {
                let mut payload = SetIrqsPayload::default();
                stream
                    .read_exact(bytes_of_mut(&mut payload))
                    .map_err(Error::StreamRead)?;

                if payload.index as usize >= self.irqs.len() {
                    return Err(Error::InvalidInput);
                }

                if payload.flags & VFIO_IRQ_SET_DATA_BOOL > 0 {
                    return Err(Error::UnsupportedFeature);
                }

                backend
                    .set_irqs(
                        payload.index,
                        payload.flags,
                        payload.start,
                        payload.count,
                        fds,
                    )
                    .map_err(Error::Backend)?;

                let reply = Header {
                    message_id: header.message_id,
                    command: Command::SetIrqs.into(),
                    flags: HeaderFlags::reply().into(),
                    message_size: size_of::<Header>() as u32,
                    ..Default::default()
                };
                stream
                    .write_all(bytes_of(&reply))
                    .map_err(Error::StreamWrite)?;
            }
            Command::RegionRead => {
                let mut payload = RegionAccessPayload::default();
                stream
                    .read_exact(bytes_of_mut(&mut payload))
                    .map_err(Error::StreamRead)?;

                if payload.region as usize >= self.regions.len() {
                    return Err(Error::InvalidInput);
                }

                let mut data = vec![0u8; payload.count as usize];
                backend
                    .region_read(payload.region, payload.offset, &mut data)
                    .map_err(Error::Backend)?;

                let reply = RegionAccess {
                    header: Header {
                        message_id: header.message_id,
                        command: Command::RegionRead.into(),
                        flags: HeaderFlags::reply().into(),
                        message_size: size_of::<RegionAccess>() as u32 + payload.count,
                        ..Default::default()
                    },
                    payload,
                };
                stream
                    .write_all(bytes_of(&reply))
                    .map_err(Error::StreamWrite)?;
                stream.write_all(&data).map_err(Error::StreamWrite)?;
            }
            Command::RegionWrite => {
                let mut payload = RegionAccessPayload::default();
                stream
                    .read_exact(bytes_of_mut(&mut payload))
                    .map_err(Error::StreamRead)?;

                let (region, offset, count) = (payload.region, payload.offset, payload.count);

                if region as usize >= self.regions.len() {
                    return Err(Error::InvalidInput);
                }

                let mut data = vec![0u8; count as usize];
                stream.read_exact(&mut data).map_err(Error::StreamRead)?;
                backend
                    .region_write(region, offset, &data)
                    .map_err(Error::Backend)?;

                let reply = RegionAccess {
                    header: Header {
                        message_id: header.message_id,
                        command: Command::RegionWrite.into(),
                        flags: HeaderFlags::reply().into(),
                        message_size: size_of::<RegionAccess>() as u32,
                        ..Default::default()
                    },
                    payload: RegionAccessPayload {
                        region,
                        offset,
                        count,
                    },
                };
                stream
                    .write_all(bytes_of(&reply))
                    .map_err(Error::StreamWrite)?;
            }
            Command::DeviceReset => {
                backend.reset().map_err(Error::Backend)?;
                let reply = Header {
                    message_id: header.message_id,
                    command: Command::DeviceReset.into(),
                    flags: HeaderFlags::reply().into(),
                    message_size: size_of::<Header>() as u32,
                    ..Default::default()
                };
                stream
                    .write_all(bytes_of(&reply))
                    .map_err(Error::StreamWrite)?;
            }
        }

        Ok(())
    }

    pub fn run(&self, backend: &mut dyn ServerBackend) -> Result<(), Error> {
        let (mut stream, _) = self.listener.accept().map_err(Error::SocketAccept)?;

        loop {
            let mut header = Header::default();

            // The maximum number of FDs that can be sent is 16 so that is
            // also the maximum that can be received.
            let mut fds = [-1; 16];
            let (bytes, fds_received) = stream
                .recvmsg_fds(bytes_of_mut(&mut header), &mut fds)
                .map_err(Error::StreamRead)?;
            assert!(fds_received <= fds.len());

            // Other end closed connection
            if bytes == 0 {
                info!("Connection closed");
                break;
            }

            let files = fds[..fds_received]
                .iter()
                .map(|fd| unsafe { File::from_raw_fd(*fd) })
                .collect();

            if let Err(e) = self.handle_command(backend, &mut stream, header, files) {
                error!("Error handling command: {:?}: {e}", header.command);
                let reply = Header {
                    message_id: header.message_id,
                    command: header.command,
                    flags: HeaderFlags::reply().with_error(true).into(),
                    message_size: size_of::<Header>() as u32,
                    error: if matches!(e, Error::InvalidInput) {
                        EINVAL as u32
                    } else {
                        0
                    },
                };
                stream
                    .write_all(bytes_of(&reply))
                    .map_err(Error::StreamWrite)?;
            }
        }

        Ok(())
    }
}
