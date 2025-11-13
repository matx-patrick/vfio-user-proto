// Copyright © 2021 Intel Corporation
//
// SPDX-License-Identifier: Apache-2.0
//

use bitflags::bitflags;
use serde::{Deserialize, Serialize};
use strum::FromRepr;
use vfio_bindings::*;
use zerocopy::{FromBytes, Immutable, IntoBytes};

#[repr(u16)]
#[derive(Clone, Copy, Debug, Default, FromRepr)]
pub enum Command {
    #[default]
    Unknown = 0,
    Version = 1,
    DmaMap = 2,
    DmaUnmap = 3,
    DeviceGetInfo = 4,
    DeviceGetRegionInfo = 5,
    GetRegionIoFds = 6,
    GetIrqInfo = 7,
    SetIrqs = 8,
    RegionRead = 9,
    RegionWrite = 10,
    DmaRead = 11,
    DmaWrite = 12,
    DeviceReset = 13,
    UserDirtyPages = 14,
}
impl From<Command> for u16 {
    fn from(value: Command) -> Self {
        value as u16
    }
}
impl TryFrom<u16> for Command {
    type Error = u16;

    fn try_from(value: u16) -> Result<Self, Self::Error> {
        Command::from_repr(value).ok_or(value)
    }
}

const HEADER_MASK_TYPE: u32 = 0b111;
const HEADER_FLAG_NO_REPLY: u32 = 1 << 4;
const HEADER_FLAG_ERROR: u32 = 1 << 5;

#[derive(FromRepr)]
#[repr(u8)]
pub enum HeaderType {
    Command = 0,
    Reply = 1,
}
pub struct HeaderFlags {
    pub ty: HeaderType,
    pub no_reply: bool,
    pub error: bool,
}
impl HeaderFlags {
    pub const fn command() -> Self {
        Self {
            ty: HeaderType::Command,
            no_reply: false,
            error: false,
        }
    }
    pub const fn reply() -> Self {
        Self {
            ty: HeaderType::Reply,
            no_reply: false,
            error: false,
        }
    }
    pub const fn with_error(self, error: bool) -> Self {
        Self { error, ..self }
    }
    pub const fn with_no_reply(self, no_reply: bool) -> Self {
        Self { no_reply, ..self }
    }
}
impl TryFrom<u32> for HeaderFlags {
    type Error = u32;

    fn try_from(value: u32) -> Result<Self, Self::Error> {
        let ty = HeaderType::from_repr((value & HEADER_MASK_TYPE) as u8).ok_or(value)?;
        Ok(Self {
            ty,
            no_reply: (value & HEADER_FLAG_NO_REPLY) != 0,
            error: (value & HEADER_FLAG_ERROR) != 0,
        })
    }
}
impl From<HeaderFlags> for u32 {
    fn from(value: HeaderFlags) -> Self {
        (value.ty as u32)
            | value.no_reply.then_some(HEADER_FLAG_NO_REPLY).unwrap_or(0)
            | value.error.then_some(HEADER_FLAG_ERROR).unwrap_or(0)
    }
}

#[repr(C)]
#[derive(Default, Clone, Copy, Debug, FromBytes, IntoBytes, Immutable)]
pub struct Header {
    pub message_id: u16,
    pub command: u16,
    pub message_size: u32,
    pub flags: u32,
    pub error: u32,
}

#[repr(C)]
#[derive(Default, Clone, Copy, Debug, FromBytes, IntoBytes, Immutable)]
pub struct Version {
    pub header: Header,
    pub major: u16,
    pub minor: u16,
}

#[derive(Serialize, Deserialize, Debug, FromBytes, IntoBytes, Immutable)]
pub struct MigrationCapabilities {
    pub pgsize: u32,
}

bitflags! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct DmaMapFlags: u32 {
        const READ = 1 << 0;
        const WRITE = 1 << 1;
        const READ_WRITE = Self::READ.bits() | Self::WRITE.bits();

        // There might be unknown bits and we don't want bitflags to clear them.
        const _ = !0;
    }

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct DmaUnmapFlags: u32 {
        const GET_DIRTY_PAGE_INFO = 1 << 1;
        const UNMAP_ALL = 1 << 2;

        // See above.
        const _ = !0;
    }
}

#[repr(C)]
#[derive(Default, Clone, Copy, Debug, FromBytes, IntoBytes, Immutable)]
pub struct DmaMap {
    pub header: Header,
    pub argsz: u32,
    pub flags: u32,
    pub offset: u64,
    pub address: u64,
    pub size: u64,
}

#[repr(C)]
#[derive(Default, Clone, Copy, Debug, FromBytes, IntoBytes, Immutable)]
pub struct DmaUnmap {
    pub header: Header,
    pub argsz: u32,
    pub flags: u32,
    pub address: u64,
    pub size: u64,
}

#[repr(C)]
#[derive(Default, Clone, Copy, Debug, FromBytes, IntoBytes, Immutable)]
pub struct DeviceGetInfo {
    pub header: Header,
    pub argsz: u32,
    pub flags: u32,
    pub num_regions: u32,
    pub num_irqs: u32,
}

#[repr(C)]
#[derive(Default, Clone, Copy, Debug, FromBytes, IntoBytes, Immutable)]
pub struct DeviceGetRegionInfo {
    pub header: Header,
    pub region_info: vfio_region_info,
}

#[repr(C)]
#[derive(Default, Clone, Copy, Debug, FromBytes, IntoBytes, Immutable)]
pub struct RegionAccess {
    pub header: Header,
    pub offset: u64,
    pub region: u32,
    pub count: u32,
}

#[repr(C)]
#[derive(Default, Clone, Copy, Debug, FromBytes, IntoBytes, Immutable)]
pub struct GetIrqInfo {
    pub header: Header,
    pub argsz: u32,
    pub flags: u32,
    pub index: u32,
    pub count: u32,
}

#[repr(C)]
#[derive(Default, Clone, Copy, Debug, FromBytes, IntoBytes, Immutable)]
pub struct SetIrqs {
    pub header: Header,
    pub argsz: u32,
    pub flags: u32,
    pub index: u32,
    pub start: u32,
    pub count: u32,
}

#[repr(C)]
#[derive(Default, Clone, Copy, Debug, FromBytes, IntoBytes, Immutable)]
pub struct DeviceReset {
    pub header: Header,
}
