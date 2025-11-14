// Copyright 2019 Intel Corporation. All Rights Reserved.
// SPDX-License-Identifier: (BSD-3-Clause OR Apache-2.0)

#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]

use ::std::os::raw::c_uint;
use zerocopy::{FromBytes, Immutable, IntoBytes};

pub const VFIO_API_VERSION: u32 = 0;
pub const VFIO_TYPE1_IOMMU: u32 = 1;
pub const VFIO_SPAPR_TCE_IOMMU: u32 = 2;
pub const VFIO_TYPE1v2_IOMMU: u32 = 3;
pub const VFIO_DMA_CC_IOMMU: u32 = 4;
pub const VFIO_EEH: u32 = 5;
pub const VFIO_TYPE1_NESTING_IOMMU: u32 = 6;
pub const VFIO_SPAPR_TCE_v2_IOMMU: u32 = 7;
pub const VFIO_NOIOMMU_IOMMU: u32 = 8;
pub const VFIO_UNMAP_ALL: u32 = 9;
pub const VFIO_UPDATE_VADDR: u32 = 10;
pub const VFIO_TYPE: u8 = 59u8;
pub const VFIO_BASE: u32 = 100;
pub const VFIO_GROUP_FLAGS_VIABLE: u32 = 1;
pub const VFIO_GROUP_FLAGS_CONTAINER_SET: u32 = 2;
pub const VFIO_DEVICE_FLAGS_RESET: u32 = 1;
pub const VFIO_DEVICE_FLAGS_PCI: u32 = 2;
pub const VFIO_DEVICE_FLAGS_PLATFORM: u32 = 4;
pub const VFIO_DEVICE_FLAGS_AMBA: u32 = 8;
pub const VFIO_DEVICE_FLAGS_CCW: u32 = 16;
pub const VFIO_DEVICE_FLAGS_AP: u32 = 32;
pub const VFIO_DEVICE_FLAGS_FSL_MC: u32 = 64;
pub const VFIO_DEVICE_FLAGS_CAPS: u32 = 128;
pub const VFIO_DEVICE_FLAGS_CDX: u32 = 256;
pub const VFIO_DEVICE_API_PCI_STRING: &[u8; 9] = b"vfio-pci\0";
pub const VFIO_DEVICE_API_PLATFORM_STRING: &[u8; 14] = b"vfio-platform\0";
pub const VFIO_DEVICE_API_AMBA_STRING: &[u8; 10] = b"vfio-amba\0";
pub const VFIO_DEVICE_API_CCW_STRING: &[u8; 9] = b"vfio-ccw\0";
pub const VFIO_DEVICE_API_AP_STRING: &[u8; 8] = b"vfio-ap\0";
pub const VFIO_DEVICE_INFO_CAP_ZPCI_BASE: u32 = 1;
pub const VFIO_DEVICE_INFO_CAP_ZPCI_GROUP: u32 = 2;
pub const VFIO_DEVICE_INFO_CAP_ZPCI_UTIL: u32 = 3;
pub const VFIO_DEVICE_INFO_CAP_ZPCI_PFIP: u32 = 4;
pub const VFIO_DEVICE_INFO_CAP_PCI_ATOMIC_COMP: u32 = 5;
pub const VFIO_PCI_ATOMIC_COMP32: u32 = 1;
pub const VFIO_PCI_ATOMIC_COMP64: u32 = 2;
pub const VFIO_PCI_ATOMIC_COMP128: u32 = 4;
pub const VFIO_REGION_INFO_FLAG_READ: u32 = 1;
pub const VFIO_REGION_INFO_FLAG_WRITE: u32 = 2;
pub const VFIO_REGION_INFO_FLAG_MMAP: u32 = 4;
pub const VFIO_REGION_INFO_FLAG_CAPS: u32 = 8;
pub const VFIO_REGION_INFO_CAP_SPARSE_MMAP: u32 = 1;
pub const VFIO_REGION_INFO_CAP_TYPE: u32 = 2;
pub const VFIO_REGION_TYPE_PCI_VENDOR_TYPE: u32 = 2147483648;
pub const VFIO_REGION_TYPE_PCI_VENDOR_MASK: u32 = 65535;
pub const VFIO_REGION_TYPE_GFX: u32 = 1;
pub const VFIO_REGION_TYPE_CCW: u32 = 2;
pub const VFIO_REGION_TYPE_MIGRATION_DEPRECATED: u32 = 3;
pub const VFIO_REGION_SUBTYPE_INTEL_IGD_OPREGION: u32 = 1;
pub const VFIO_REGION_SUBTYPE_INTEL_IGD_HOST_CFG: u32 = 2;
pub const VFIO_REGION_SUBTYPE_INTEL_IGD_LPC_CFG: u32 = 3;
pub const VFIO_REGION_SUBTYPE_NVIDIA_NVLINK2_RAM: u32 = 1;
pub const VFIO_REGION_SUBTYPE_IBM_NVLINK2_ATSD: u32 = 1;
pub const VFIO_REGION_SUBTYPE_GFX_EDID: u32 = 1;
pub const VFIO_DEVICE_GFX_LINK_STATE_UP: u32 = 1;
pub const VFIO_DEVICE_GFX_LINK_STATE_DOWN: u32 = 2;
pub const VFIO_REGION_SUBTYPE_CCW_ASYNC_CMD: u32 = 1;
pub const VFIO_REGION_SUBTYPE_CCW_SCHIB: u32 = 2;
pub const VFIO_REGION_SUBTYPE_CCW_CRW: u32 = 3;
pub const VFIO_REGION_SUBTYPE_MIGRATION_DEPRECATED: u32 = 1;
pub const VFIO_DEVICE_STATE_V1_STOP: u32 = 0;
pub const VFIO_DEVICE_STATE_V1_RUNNING: u32 = 1;
pub const VFIO_DEVICE_STATE_V1_SAVING: u32 = 2;
pub const VFIO_DEVICE_STATE_V1_RESUMING: u32 = 4;
pub const VFIO_DEVICE_STATE_MASK: u32 = 7;
pub const VFIO_REGION_INFO_CAP_MSIX_MAPPABLE: u32 = 3;
pub const VFIO_REGION_INFO_CAP_NVLINK2_SSATGT: u32 = 4;
pub const VFIO_REGION_INFO_CAP_NVLINK2_LNKSPD: u32 = 5;
pub const VFIO_IRQ_INFO_EVENTFD: u32 = 1;
pub const VFIO_IRQ_INFO_MASKABLE: u32 = 2;
pub const VFIO_IRQ_INFO_AUTOMASKED: u32 = 4;
pub const VFIO_IRQ_INFO_NORESIZE: u32 = 8;
pub const VFIO_IRQ_SET_DATA_NONE: u32 = 1;
pub const VFIO_IRQ_SET_DATA_BOOL: u32 = 2;
pub const VFIO_IRQ_SET_DATA_EVENTFD: u32 = 4;
pub const VFIO_IRQ_SET_ACTION_MASK: u32 = 8;
pub const VFIO_IRQ_SET_ACTION_UNMASK: u32 = 16;
pub const VFIO_IRQ_SET_ACTION_TRIGGER: u32 = 32;
pub const VFIO_IRQ_SET_DATA_TYPE_MASK: u32 = 7;
pub const VFIO_IRQ_SET_ACTION_TYPE_MASK: u32 = 56;
pub const VFIO_PCI_DEVID_OWNED: u32 = 0;
pub const VFIO_PCI_DEVID_NOT_OWNED: i32 = -1;
pub const VFIO_PCI_HOT_RESET_FLAG_DEV_ID: u32 = 1;
pub const VFIO_PCI_HOT_RESET_FLAG_DEV_ID_OWNED: u32 = 2;
pub const VFIO_GFX_PLANE_TYPE_PROBE: u32 = 1;
pub const VFIO_GFX_PLANE_TYPE_DMABUF: u32 = 2;
pub const VFIO_GFX_PLANE_TYPE_REGION: u32 = 4;
pub const VFIO_DEVICE_IOEVENTFD_8: u32 = 1;
pub const VFIO_DEVICE_IOEVENTFD_16: u32 = 2;
pub const VFIO_DEVICE_IOEVENTFD_32: u32 = 4;
pub const VFIO_DEVICE_IOEVENTFD_64: u32 = 8;
pub const VFIO_DEVICE_IOEVENTFD_SIZE_MASK: u32 = 15;
pub const VFIO_DEVICE_FEATURE_MASK: u32 = 65535;
pub const VFIO_DEVICE_FEATURE_GET: u32 = 65536;
pub const VFIO_DEVICE_FEATURE_SET: u32 = 131072;
pub const VFIO_DEVICE_FEATURE_PROBE: u32 = 262144;
pub const VFIO_DEVICE_FEATURE_PCI_VF_TOKEN: u32 = 0;
pub const VFIO_MIGRATION_STOP_COPY: u32 = 1;
pub const VFIO_MIGRATION_P2P: u32 = 2;
pub const VFIO_MIGRATION_PRE_COPY: u32 = 4;
pub const VFIO_DEVICE_FEATURE_MIGRATION: u32 = 1;
pub const VFIO_DEVICE_FEATURE_MIG_DEVICE_STATE: u32 = 2;
pub const VFIO_DEVICE_FEATURE_LOW_POWER_ENTRY: u32 = 3;
pub const VFIO_DEVICE_FEATURE_LOW_POWER_ENTRY_WITH_WAKEUP: u32 = 4;
pub const VFIO_DEVICE_FEATURE_LOW_POWER_EXIT: u32 = 5;
pub const VFIO_DEVICE_FEATURE_DMA_LOGGING_START: u32 = 6;
pub const VFIO_DEVICE_FEATURE_DMA_LOGGING_STOP: u32 = 7;
pub const VFIO_DEVICE_FEATURE_DMA_LOGGING_REPORT: u32 = 8;
pub const VFIO_DEVICE_FEATURE_MIG_DATA_SIZE: u32 = 9;
pub const VFIO_IOMMU_INFO_PGSIZES: u32 = 1;
pub const VFIO_IOMMU_INFO_CAPS: u32 = 2;
pub const VFIO_IOMMU_TYPE1_INFO_CAP_IOVA_RANGE: u32 = 1;
pub const VFIO_IOMMU_TYPE1_INFO_CAP_MIGRATION: u32 = 2;
pub const VFIO_IOMMU_TYPE1_INFO_DMA_AVAIL: u32 = 3;
pub const VFIO_DMA_MAP_FLAG_READ: u32 = 1;
pub const VFIO_DMA_MAP_FLAG_WRITE: u32 = 2;
pub const VFIO_DMA_MAP_FLAG_VADDR: u32 = 4;
pub const VFIO_DMA_UNMAP_FLAG_GET_DIRTY_BITMAP: u32 = 1;
pub const VFIO_DMA_UNMAP_FLAG_ALL: u32 = 2;
pub const VFIO_DMA_UNMAP_FLAG_VADDR: u32 = 4;
pub const VFIO_IOMMU_DIRTY_PAGES_FLAG_START: u32 = 1;
pub const VFIO_IOMMU_DIRTY_PAGES_FLAG_STOP: u32 = 2;
pub const VFIO_IOMMU_DIRTY_PAGES_FLAG_GET_BITMAP: u32 = 4;
pub const VFIO_IOMMU_SPAPR_INFO_DDW: u32 = 1;
pub const VFIO_EEH_PE_DISABLE: u32 = 0;
pub const VFIO_EEH_PE_ENABLE: u32 = 1;
pub const VFIO_EEH_PE_UNFREEZE_IO: u32 = 2;
pub const VFIO_EEH_PE_UNFREEZE_DMA: u32 = 3;
pub const VFIO_EEH_PE_GET_STATE: u32 = 4;
pub const VFIO_EEH_PE_STATE_NORMAL: u32 = 0;
pub const VFIO_EEH_PE_STATE_RESET: u32 = 1;
pub const VFIO_EEH_PE_STATE_STOPPED: u32 = 2;
pub const VFIO_EEH_PE_STATE_STOPPED_DMA: u32 = 4;
pub const VFIO_EEH_PE_STATE_UNAVAIL: u32 = 5;
pub const VFIO_EEH_PE_RESET_DEACTIVATE: u32 = 5;
pub const VFIO_EEH_PE_RESET_HOT: u32 = 6;
pub const VFIO_EEH_PE_RESET_FUNDAMENTAL: u32 = 7;
pub const VFIO_EEH_PE_CONFIGURE: u32 = 8;
pub const VFIO_EEH_PE_INJECT_ERR: u32 = 9;

pub const VFIO_PCI_BAR0_REGION_INDEX: c_uint = 0;
pub const VFIO_PCI_BAR1_REGION_INDEX: c_uint = 1;
pub const VFIO_PCI_BAR2_REGION_INDEX: c_uint = 2;
pub const VFIO_PCI_BAR3_REGION_INDEX: c_uint = 3;
pub const VFIO_PCI_BAR4_REGION_INDEX: c_uint = 4;
pub const VFIO_PCI_BAR5_REGION_INDEX: c_uint = 5;
pub const VFIO_PCI_ROM_REGION_INDEX: c_uint = 6;
pub const VFIO_PCI_CONFIG_REGION_INDEX: c_uint = 7;
pub const VFIO_PCI_VGA_REGION_INDEX: c_uint = 8;
pub const VFIO_PCI_NUM_REGIONS: c_uint = 9;
pub const VFIO_PCI_INTX_IRQ_INDEX: c_uint = 0;
pub const VFIO_PCI_MSI_IRQ_INDEX: c_uint = 1;
pub const VFIO_PCI_MSIX_IRQ_INDEX: c_uint = 2;
pub const VFIO_PCI_ERR_IRQ_INDEX: c_uint = 3;
pub const VFIO_PCI_REQ_IRQ_INDEX: c_uint = 4;
pub const VFIO_PCI_NUM_IRQS: c_uint = 5;

#[repr(C)]
#[derive(Default)]
pub struct IncompleteArrayField<T>(::std::marker::PhantomData<T>, [T; 0]);
impl<T> IncompleteArrayField<T> {
    #[inline]
    pub const fn new() -> Self {
        IncompleteArrayField(::std::marker::PhantomData, [])
    }
    #[inline]
    pub fn as_ptr(&self) -> *const T {
        self as *const _ as *const T
    }
    #[inline]
    pub fn as_mut_ptr(&mut self) -> *mut T {
        self as *mut _ as *mut T
    }
    #[inline]
    pub unsafe fn as_slice(&self, len: usize) -> &[T] {
        ::std::slice::from_raw_parts(self.as_ptr(), len)
    }
    #[inline]
    pub unsafe fn as_mut_slice(&mut self, len: usize) -> &mut [T] {
        ::std::slice::from_raw_parts_mut(self.as_mut_ptr(), len)
    }
}
impl<T> ::std::fmt::Debug for IncompleteArrayField<T> {
    fn fmt(&self, fmt: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
        fmt.write_str("__IncompleteArrayField")
    }
}

#[repr(C)]
#[derive(Debug, Default, Copy, Clone, PartialEq)]
pub struct vfio_info_cap_header {
    pub id: u16,
    pub version: u16,
    pub next: u32,
}

#[repr(C)]
#[derive(Debug, Default, Copy, Clone, PartialEq)]
pub struct vfio_group_status {
    pub argsz: u32,
    pub flags: u32,
}

#[repr(C)]
#[derive(Debug, Default, Copy, Clone, PartialEq)]
pub struct vfio_device_info {
    pub argsz: u32,
    pub flags: u32,
    pub num_regions: u32,
    pub num_irqs: u32,
    pub cap_offset: u32,
    pub pad: u32,
}

#[repr(C)]
#[derive(Debug, Default, Copy, Clone, PartialEq)]
pub struct vfio_device_info_cap_pci_atomic_comp {
    pub header: vfio_info_cap_header,
    pub flags: u32,
    pub reserved: u32,
}

#[repr(C)]
#[derive(Debug, Default, Copy, Clone, PartialEq, FromBytes, IntoBytes, Immutable)]
pub struct vfio_region_info {
    pub argsz: u32,
    pub flags: u32,
    pub index: u32,
    pub cap_offset: u32,
    pub size: u64,
    pub offset: u64,
}

#[repr(C)]
#[derive(Debug, Default, Copy, Clone, PartialEq)]
pub struct vfio_region_sparse_mmap_area {
    pub offset: u64,
    pub size: u64,
}

#[repr(C)]
#[derive(Debug, Default)]
pub struct vfio_region_info_cap_sparse_mmap {
    pub header: vfio_info_cap_header,
    pub nr_areas: u32,
    pub reserved: u32,
    pub areas: IncompleteArrayField<vfio_region_sparse_mmap_area>,
}

#[repr(C)]
#[derive(Debug, Default, Copy, Clone, PartialEq)]
pub struct vfio_region_info_cap_type {
    pub header: vfio_info_cap_header,
    pub type_: u32,
    pub subtype: u32,
}

#[repr(C)]
#[derive(Debug, Default, Copy, Clone, PartialEq)]
pub struct vfio_region_gfx_edid {
    pub edid_offset: u32,
    pub edid_max_size: u32,
    pub edid_size: u32,
    pub max_xres: u32,
    pub max_yres: u32,
    pub link_state: u32,
}

#[repr(C)]
#[derive(Debug, Default, Copy, Clone, PartialEq)]
pub struct vfio_device_migration_info {
    pub device_state: u32,
    pub reserved: u32,
    pub pending_bytes: u64,
    pub data_offset: u64,
    pub data_size: u64,
}

#[repr(C)]
#[derive(Debug, Default, Copy, Clone, PartialEq)]
pub struct vfio_region_info_cap_nvlink2_ssatgt {
    pub header: vfio_info_cap_header,
    pub tgt: u64,
}

#[repr(C)]
#[derive(Debug, Default, Copy, Clone, PartialEq)]
pub struct vfio_region_info_cap_nvlink2_lnkspd {
    pub header: vfio_info_cap_header,
    pub link_speed: u32,
    pub __pad: u32,
}

#[repr(C)]
#[derive(Debug, Default, Copy, Clone, PartialEq)]
pub struct vfio_irq_info {
    pub argsz: u32,
    pub flags: u32,
    pub index: u32,
    pub count: u32,
}

#[repr(C)]
#[derive(Debug, Default, Copy, Clone, PartialEq)]
pub struct vfio_device_ioeventfd {
    pub argsz: u32,
    pub flags: u32,
    pub offset: u64,
    pub data: u64,
    pub fd: i32,
}
