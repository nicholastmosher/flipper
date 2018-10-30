use std::slice;
use std::mem::size_of;
use std::os::raw::c_char;
use std::fmt::{self as fmt, Debug};

pub const FMR_MAGIC_NUMBER: u8 = 0xFE;
pub const FMR_PACKET_SIZE: usize = 64;
pub const FMR_PAYLOAD_SIZE: usize = FMR_PACKET_SIZE - size_of::<FmrHeader>();

#[derive(Copy, Clone)]
#[repr(C, packed)]
pub struct FmrPayload([u8; FMR_PAYLOAD_SIZE]);
const FMR_PAYLOAD_EMPTY: FmrPayload = FmrPayload([0; FMR_PAYLOAD_SIZE]);

pub type LfCrc = u16;
pub type LfTypes = u32;
pub type LfValue = u64;
pub type LfArgc = u8;
pub type LfArgRepr = u64;
pub type LfModule = u32;
pub type LfFunction = u8;
pub type LfAddress = u32;

#[derive(Copy, Clone)]
pub struct LfPointer(pub(crate) LfAddress);

#[derive(Debug, Copy, Clone)]
#[repr(u8)]
pub enum FmrClass {
    call = 0,
    push = 1,
    pull = 2,
    dyld = 3,
    malloc = 4,
    free = 5,
}

#[derive(Debug, Copy, Clone)]
#[repr(C, packed)]
pub struct FmrHeader {
    pub magic: u8,
    pub crc: LfCrc,
    pub len: u16,
    pub kind: FmrClass,
}

pub union FmrPacket {
    pub base: FmrPacketBase,
    pub call: FmrPacketCall,
    pub data: FmrPacketPushPull,
    pub dyld: FmrPacketDyld,
    pub memory: FmrPacketMemory,
}

impl FmrPacket {
    pub fn new(class: FmrClass) -> FmrPacket {
        FmrPacket {
            base: FmrPacketBase {
                header: FmrHeader {
                    magic: FMR_MAGIC_NUMBER,
                    crc: 0,
                    // Under normal circumstances this would be mem::size_of::<FmrHeader>(),
                    // but for some reason the packed repr in C calculates the size as 8, not 6.
                    len: 8,
                    kind: class,
                },
                payload: FMR_PAYLOAD_EMPTY,
            }
        }
    }
}

pub trait FmrAsBytes: Sized {
    unsafe fn as_bytes(&self) -> &[u8] {
        slice::from_raw_parts(self as *const _ as *const u8, size_of::<FmrPacket>())
    }
}

impl From<FmrPacket> for FmrPacketCall {
    fn from(mut packet: FmrPacket) -> Self {
        unsafe { *(&mut packet as *mut _ as *mut FmrPacketCall) }
    }
}

impl FmrAsBytes for FmrPacketCall { }

impl From<FmrPacket> for FmrPacketPushPull {
    fn from(mut packet: FmrPacket) -> Self {
        unsafe { *(&mut packet as *mut _ as *mut FmrPacketPushPull) }
    }
}

impl FmrAsBytes for FmrPacketPushPull { }

impl From<FmrPacket> for FmrPacketDyld {
    fn from(mut packet: FmrPacket) -> Self {
        unsafe { *(&mut packet as *mut _ as *mut FmrPacketDyld) }
    }
}

impl FmrAsBytes for FmrPacketDyld { }

impl From<FmrPacket> for FmrPacketMemory {
    fn from(mut packet: FmrPacket) -> Self {
        unsafe { *(&mut packet as *mut _ as *mut FmrPacketMemory) }
    }
}

impl FmrAsBytes for FmrPacketMemory { }

#[derive(Debug, Copy, Clone)]
#[repr(C, packed)]
pub struct FmrPacketBase {
    pub header: FmrHeader,
    pub payload: FmrPayload,
}

#[derive(Debug, Copy, Clone)]
#[repr(C, packed)]
pub struct FmrCall {
    pub module: u8,
    pub function: u8,
    pub ret: LfType,
    pub argt: LfTypes,
    pub argc: LfArgc,
    pub argv: (),
}

#[derive(Debug, Copy, Clone)]
#[repr(C, packed)]
pub struct FmrPacketCall {
    pub header: FmrHeader,
    pub call: FmrCall,
}

#[derive(Debug, Copy, Clone)]
#[repr(C, packed)]
pub struct FmrPacketPushPull {
    pub header: FmrHeader,
    pub len: u32,
    pub ptr: u64,
}

#[derive(Debug, Copy, Clone)]
#[repr(C, packed)]
pub struct FmrPacketDyld {
    pub header: FmrHeader,
    pub module: *mut c_char,
}

#[derive(Debug, Copy, Clone)]
#[repr(C, packed)]
pub struct FmrPacketMemory {
    pub header: FmrHeader,
    pub size: u32,
    pub ptr: u64,
}

#[derive(Debug, Copy, Clone)]
#[repr(C, packed)]
pub struct FmrReturn {
    pub value: LfValue,
    pub error: u8,
}

impl FmrReturn {
    pub fn new() -> FmrReturn { FmrReturn { value: 0, error: 0 } }

    pub unsafe fn as_mut_bytes(&mut self) -> &mut [u8] {
        slice::from_raw_parts_mut(self as *mut _ as *mut u8, size_of::<FmrReturn>())
    }
}

#[derive(Debug, Copy, Clone)]
#[repr(C, packed)]
pub struct LfArg {
    pub kind: LfType,
    pub value: LfArgRepr,
}

#[derive(Debug, Copy, Clone)]
#[repr(u8)]
pub enum LfType {
    lf_void = 2,
    lf_int = 4,
    lf_ptr = 6,

    // Unsigned types
    lf_uint8 = 0,
    lf_uint16 = 1,
    lf_uint32 = 3,
    lf_uint64 = 7,

    // Signed types
    lf_int8 = 8,
    lf_int16 = 9,
    lf_int32 = 11,
    lf_int64 = 15,
}

impl LfType {
    pub const MAX: u8 = 15;

    pub fn size(&self) -> usize {
        match self {
            LfType::lf_int8 | LfType::lf_uint8 => 1,
            LfType::lf_int16 | LfType::lf_uint16 => 2,
            LfType::lf_int32 | LfType::lf_uint32 => 4,
            LfType::lf_int64 |
            LfType::lf_uint64 |
            LfType::lf_ptr |
            LfType::lf_void => 8,
            _ => 0,
        }
    }

    pub fn from(byte: u8) -> Option<LfType> {
        match byte {
            0 => Some(LfType::lf_uint8),
            1 => Some(LfType::lf_uint16),
            2 => Some(LfType::lf_void),
            3 => Some(LfType::lf_uint32),
            4 => Some(LfType::lf_int),
            6 => Some(LfType::lf_ptr),
            7 => Some(LfType::lf_uint64),
            8 => Some(LfType::lf_int8),
            9 => Some(LfType::lf_int16),
            11 => Some(LfType::lf_int32),
            15 => Some(LfType::lf_int64),
            _ => None
        }
    }
}

impl Debug for FmrPayload {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        for chunk in self.0.chunks(8) {
            for byte in chunk { write!(f, "{:02X} ", byte)?; }
            writeln!(f);
        }
        Ok(())
    }
}
