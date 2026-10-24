#![cfg(target_os = "windows")]

use super::audio_bridge::BRIDGE_PROTOCOL_VERSION;
use super::{AudioBridge, BridgeAudioFormat, BridgeStatus, BridgeWriteOutcome, VirtualAudioError};
use std::{ffi::c_void, mem::size_of, ptr};

const PACKET_FRAMES: usize = 480;
const INVALID_HANDLE_VALUE: isize = -1;
const fn ctl(function: u32) -> u32 {
    (0x8337 << 16) | (3 << 14) | (function << 2)
}
const IOCTL_HELLO: u32 = ctl(0x800);
const IOCTL_CONFIGURE: u32 = ctl(0x801);
const IOCTL_WRITE: u32 = ctl(0x802);
const IOCTL_STATUS: u32 = ctl(0x803);
const IOCTL_RESET: u32 = ctl(0x804);

#[repr(C)]
#[derive(Clone, Copy)]
struct Guid {
    d1: u32,
    d2: u16,
    d3: u16,
    d4: [u8; 8],
}
const INTERFACE: Guid = Guid {
    d1: 0xb566f7b7,
    d2: 0x91d8,
    d3: 0x4f31,
    d4: [0x97, 0x12, 0x94, 0xc5, 0x5a, 0x10, 0xe5, 0x2a],
};
#[repr(C)]
struct Format {
    sample_rate: u32,
    channels: u16,
    sample_format: u16,
    frames_per_packet: u32,
}
#[repr(C)]
#[derive(Default)]
struct Counters {
    written: u64,
    consumed: u64,
    dropped: u64,
    underrun: u64,
}
#[repr(C)]
#[derive(Default)]
struct Status {
    version: u32,
    configured: u32,
    queued: u32,
    capacity: u32,
    version_major: u16,
    version_minor: u16,
    version_patch: u16,
    version_build: u16,
    reserved: [u32; 2],
    counters: Counters,
}
#[repr(C)]
struct Header {
    version: u32,
    message_type: u32,
    payload_bytes: u32,
    reserved: u32,
    sequence: u64,
}
#[repr(C)]
struct Packet {
    header: Header,
    samples: [i32; PACKET_FRAMES],
}
type Handle = *mut c_void;

#[link(name = "Cfgmgr32")]
unsafe extern "system" {
    fn CM_Get_Device_Interface_List_SizeW(
        length: *mut u32,
        guid: *const Guid,
        device_id: *const u16,
        flags: u32,
    ) -> u32;
    fn CM_Get_Device_Interface_ListW(
        guid: *const Guid,
        device_id: *const u16,
        buffer: *mut u16,
        length: u32,
        flags: u32,
    ) -> u32;
}
#[link(name = "Kernel32")]
unsafe extern "system" {
    fn CreateFileW(
        name: *const u16,
        access: u32,
        share: u32,
        security: *mut c_void,
        creation: u32,
        flags: u32,
        template: Handle,
    ) -> Handle;
    fn DeviceIoControl(
        handle: Handle,
        code: u32,
        input: *const c_void,
        input_len: u32,
        output: *mut c_void,
        output_len: u32,
        returned: *mut u32,
        overlapped: *mut c_void,
    ) -> i32;
    fn CloseHandle(handle: Handle) -> i32;
    fn GetLastError() -> u32;
}

pub struct WindowsDriverBridge {
    handle: Handle,
    sequence: u64,
    driver_version: [u16; 4],
}
unsafe impl Send for WindowsDriverBridge {}
impl WindowsDriverBridge {
    pub fn connect() -> Result<Self, VirtualAudioError> {
        let mut length = 0;
        let result =
            unsafe { CM_Get_Device_Interface_List_SizeW(&mut length, &INTERFACE, ptr::null(), 0) };
        if result != 0 || length <= 1 {
            return Err(VirtualAudioError::DriverNotInstalled);
        }
        let mut paths = vec![0_u16; length as usize];
        if unsafe {
            CM_Get_Device_Interface_ListW(&INTERFACE, ptr::null(), paths.as_mut_ptr(), length, 0)
        } != 0
        {
            return Err(VirtualAudioError::BridgeUnavailable);
        }
        let handle = unsafe {
            CreateFileW(
                paths.as_ptr(),
                0xC000_0000,
                3,
                ptr::null_mut(),
                3,
                0,
                ptr::null_mut(),
            )
        };
        if handle as isize == INVALID_HANDLE_VALUE {
            return Err(if unsafe { GetLastError() } == 5 {
                VirtualAudioError::AccessDenied
            } else {
                VirtualAudioError::BridgeUnavailable
            });
        }
        let mut bridge = Self {
            handle,
            sequence: 0,
            driver_version: [0; 4],
        };
        let status = bridge.ioctl_status(IOCTL_HELLO)?;
        if status.version != BRIDGE_PROTOCOL_VERSION {
            return Err(VirtualAudioError::IncompatibleProtocol {
                expected: BRIDGE_PROTOCOL_VERSION,
                actual: status.version,
            });
        }
        bridge.driver_version = [
            status.version_major,
            status.version_minor,
            status.version_patch,
            status.version_build,
        ];
        Ok(bridge)
    }
    pub fn driver_version(&self) -> String {
        format!(
            "{}.{}.{}.{}",
            self.driver_version[0],
            self.driver_version[1],
            self.driver_version[2],
            self.driver_version[3]
        )
    }
    fn ioctl(
        &self,
        code: u32,
        input: *const c_void,
        input_len: u32,
        output: *mut c_void,
        output_len: u32,
    ) -> Result<u32, VirtualAudioError> {
        let mut returned = 0;
        let ok = unsafe {
            DeviceIoControl(
                self.handle,
                code,
                input,
                input_len,
                output,
                output_len,
                &mut returned,
                ptr::null_mut(),
            )
        };
        if ok == 0 {
            Err(VirtualAudioError::Transport(format!(
                "DeviceIoControl 0x{code:08x} failed with Win32 error {}",
                unsafe { GetLastError() }
            )))
        } else {
            Ok(returned)
        }
    }
    fn ioctl_status(&mut self, code: u32) -> Result<Status, VirtualAudioError> {
        let mut status = Status::default();
        self.ioctl(
            code,
            ptr::null(),
            0,
            (&mut status as *mut Status).cast(),
            size_of::<Status>() as u32,
        )?;
        Ok(status)
    }
}
impl AudioBridge for WindowsDriverBridge {
    fn open(&mut self, format: BridgeAudioFormat) -> Result<(), VirtualAudioError> {
        format.validate()?;
        let wire = Format {
            sample_rate: format.sample_rate,
            channels: format.channels,
            sample_format: 2,
            frames_per_packet: PACKET_FRAMES as u32,
        };
        self.ioctl(
            IOCTL_CONFIGURE,
            (&wire as *const Format).cast(),
            size_of::<Format>() as u32,
            ptr::null_mut(),
            0,
        )
        .map(|_| ())
    }
    fn write_packet(&mut self, samples: &[i32]) -> BridgeWriteOutcome {
        if samples.len() > PACKET_FRAMES {
            return BridgeWriteOutcome::Dropped {
                frames: samples.len(),
            };
        }
        let mut packet = Packet {
            header: Header {
                version: 1,
                message_type: 3,
                payload_bytes: (samples.len() * 4) as u32,
                reserved: 0,
                sequence: self.sequence,
            },
            samples: [0; PACKET_FRAMES],
        };
        packet.samples[..samples.len()].copy_from_slice(samples);
        self.sequence = self.sequence.wrapping_add(1);
        let bytes = size_of::<Header>() + std::mem::size_of_val(samples);
        match self.ioctl(
            IOCTL_WRITE,
            (&packet as *const Packet).cast(),
            bytes as u32,
            ptr::null_mut(),
            0,
        ) {
            Ok(bytes) if bytes as usize == std::mem::size_of_val(samples) => {
                BridgeWriteOutcome::Written {
                    frames: samples.len(),
                }
            }
            Ok(bytes) => BridgeWriteOutcome::Dropped {
                frames: samples
                    .len()
                    .saturating_sub(bytes as usize / size_of::<i32>()),
            },
            Err(_) => BridgeWriteOutcome::Disconnected,
        }
    }
    fn status(&mut self) -> Result<BridgeStatus, VirtualAudioError> {
        let value = self.ioctl_status(IOCTL_STATUS)?;
        Ok(BridgeStatus {
            connected: true,
            active: value.configured != 0,
            queued_frames: value.queued,
            capacity_frames: value.capacity,
            written_frames: value.counters.written,
            dropped_frames: value.counters.dropped,
            underrun_frames: value.counters.underrun,
            transport_errors: 0,
        })
    }
    fn reset(&mut self) -> Result<(), VirtualAudioError> {
        self.ioctl(IOCTL_RESET, ptr::null(), 0, ptr::null_mut(), 0)
            .map(|_| ())
    }
    fn reconnect(&mut self) -> Result<(), VirtualAudioError> {
        self.close();
        let mut replacement = Self::connect()?;
        replacement.open(BridgeAudioFormat::PCM_S32_MONO_48K)?;
        *self = replacement;
        Ok(())
    }
    fn close(&mut self) {
        if !self.handle.is_null() && self.handle as isize != INVALID_HANDLE_VALUE {
            unsafe {
                CloseHandle(self.handle);
            }
            self.handle = ptr::null_mut();
        }
    }
}
impl Drop for WindowsDriverBridge {
    fn drop(&mut self) {
        self.close();
    }
}
