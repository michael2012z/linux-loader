// Copyright © 2020, Oracle and/or its affiliates.
//
// Copyright (c) 2019 Intel Corporation. All rights reserved.
// Copyright 2018 Amazon.com, Inc. or its affiliates. All Rights Reserved.
//
// Copyright 2017 The Chromium OS Authors. All rights reserved.
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE-BSD-3-Clause file.
//
// SPDX-License-Identifier: Apache-2.0 AND BSD-3-Clause

//! Traits and Structs
//! - [KernelLoader](trait.KernelLoader.html): load kernel image into guest memory
//! - [KernelLoaderResult](struct.KernelLoaderResult.html): the structure which loader
//! returns to VMM to assist zero page construction and boot environment setup
//! - [Elf](struct.Elf.html): elf image loader
//! - [BzImage](struct.BzImage.html): bzImage loader

extern crate vm_memory;

use std::error::{self, Error as KernelLoaderError};
use std::ffi::CStr;
use std::fmt::{self, Display};
use std::io::SeekFrom;
use std::io::{Read, Seek};
#[cfg(feature = "elf")]
#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
use std::mem;

use vm_memory::{Address, Bytes, GuestAddress, GuestMemory, GuestUsize};

#[allow(dead_code)]
#[allow(non_camel_case_types)]
#[allow(non_snake_case)]
#[allow(non_upper_case_globals)]
#[allow(missing_docs)]
#[cfg_attr(feature = "cargo-clippy", allow(clippy::all))]
#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
pub mod bootparam;

#[cfg(feature = "elf")]
#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
#[allow(missing_docs)]
#[cfg_attr(feature = "cargo-clippy", allow(clippy::all))]
pub mod start_info;

#[allow(dead_code)]
#[allow(non_camel_case_types)]
#[allow(non_snake_case)]
#[allow(non_upper_case_globals)]
#[cfg_attr(feature = "cargo-clippy", allow(clippy::all))]
#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
mod elf;
mod struct_util;

#[derive(Debug, PartialEq)]
/// Kernel loader errors.
pub enum Error {
    /// Loaded big endian binary on a little endian platform.
    BigEndianElfOnLittle,
    /// Failed writing command line to guest memory.
    CommandLineCopy,
    /// Command line overflowed guest memory.
    CommandLineOverflow,
    /// Device tree binary too big.
    #[cfg(target_arch = "aarch64")]
    DtbTooBig,
    /// Invalid ELF magic number
    InvalidElfMagicNumber,
    /// Invalid program header size.
    InvalidProgramHeaderSize,
    /// Invalid program header offset.
    InvalidProgramHeaderOffset,
    /// Invalid program header address.
    InvalidProgramHeaderAddress,
    /// Invalid entry address.
    InvalidEntryAddress,
    /// Invalid bzImage binary.
    InvalidBzImage,
    /// Invalid Image binary.
    #[cfg(target_arch = "aarch64")]
    InvalidImage,
    /// Invalid Image magic number.
    #[cfg(target_arch = "aarch64")]
    InvalidImageMagicNumber,
    /// Invalid kernel start address.
    InvalidKernelStartAddress,
    /// Memory to load kernel image is too small.
    MemoryOverflow,
    /// Unable to read ELF header.
    ReadElfHeader,
    /// Unable to read kernel image.
    ReadKernelImage,
    /// Unable to read program header.
    ReadProgramHeader,
    /// Unable to read bzImage header.
    ReadBzImageHeader,
    /// Unable to read bzImage compressed image.
    ReadBzImageCompressedKernel,
    /// Unable to read Image header
    #[cfg(target_arch = "aarch64")]
    ReadImageHeader,
    /// Unable to read DTB image
    #[cfg(target_arch = "aarch64")]
    ReadDtbImage,
    /// Unable to seek to kernel start.
    SeekKernelStart,
    /// Unable to seek to ELF start.
    SeekElfStart,
    /// Unable to seek to program header.
    SeekProgramHeader,
    /// Unable to seek to bzImage end.
    SeekBzImageEnd,
    /// Unable to seek to bzImage header.
    SeekBzImageHeader,
    /// Unable to seek to bzImage compressed kernel.
    SeekBzImageCompressedKernel,
    /// Unable to seek to Image end.
    #[cfg(target_arch = "aarch64")]
    SeekImageEnd,
    /// Unable to seek to Image header.
    #[cfg(target_arch = "aarch64")]
    SeekImageHeader,
    /// Unable to seek to DTB start.
    #[cfg(target_arch = "aarch64")]
    SeekDtbStart,
    /// Unable to seek to DTB end.
    #[cfg(target_arch = "aarch64")]
    SeekDtbEnd,
    /// Unable to seek to note header.
    SeekNoteHeader,
    /// Unable to read note header.
    ReadNoteHeader,
    /// Invalid PVH note.
    InvalidPvhNote,
}

/// A specialized `Result` type for the kernel loader.
pub type Result<T> = std::result::Result<T, Error>;

impl error::Error for Error {
    fn description(&self) -> &str {
        match self {
            Error::BigEndianElfOnLittle => {
                "Trying to load big-endian binary on little-endian machine"
            }
            Error::CommandLineCopy => "Failed writing command line to guest memory",
            Error::CommandLineOverflow => "Command line overflowed guest memory",
            #[cfg(target_arch = "aarch64")]
            Error::DtbTooBig => "Device tree image too big",
            Error::InvalidElfMagicNumber => "Invalid Elf magic number",
            Error::InvalidProgramHeaderSize => "Invalid program header size",
            Error::InvalidProgramHeaderOffset => "Invalid program header offset",
            Error::InvalidProgramHeaderAddress => "Invalid Program Header Address",
            Error::InvalidEntryAddress => "Invalid entry address",
            Error::InvalidBzImage => "Invalid bzImage",
            Error::InvalidKernelStartAddress => "Invalid kernel start address",
            #[cfg(target_arch = "aarch64")]
            Error::InvalidImage => "Invalid Image",
            #[cfg(target_arch = "aarch64")]
            Error::InvalidImageMagicNumber => "Invalid Image magic number",
            Error::MemoryOverflow => "Memory to load kernel image is not enough",
            Error::ReadElfHeader => "Unable to read elf header",
            Error::ReadKernelImage => "Unable to read kernel image",
            Error::ReadProgramHeader => "Unable to read program header",
            Error::ReadBzImageHeader => "Unable to read bzImage header",
            #[cfg(target_arch = "aarch64")]
            Error::ReadImageHeader => "Unable to read Image header",
            #[cfg(target_arch = "aarch64")]
            Error::ReadDtbImage => "Unable to read DTB image",
            Error::ReadBzImageCompressedKernel => "Unable to read bzImage compressed kernel",
            Error::SeekKernelStart => "Unable to seek to kernel start",
            Error::SeekElfStart => "Unable to seek to elf start",
            Error::SeekProgramHeader => "Unable to seek to program header",
            Error::SeekBzImageEnd => "Unable to seek bzImage end",
            Error::SeekBzImageHeader => "Unable to seek bzImage header",
            Error::SeekBzImageCompressedKernel => "Unable to seek bzImage compressed kernel",
            Error::SeekNoteHeader => "Unable to seek to note header",
            Error::ReadNoteHeader => "Unable to read note header",
            Error::InvalidPvhNote => "Invalid PVH note header",
            #[cfg(target_arch = "aarch64")]
            Error::SeekImageEnd => "Unable to seek Image end",
            #[cfg(target_arch = "aarch64")]
            Error::SeekImageHeader => "Unable to seek image header",
            #[cfg(target_arch = "aarch64")]
            Error::SeekDtbStart => "Unable to seek DTB start",
            #[cfg(target_arch = "aarch64")]
            Error::SeekDtbEnd => "Unable to seek DTB end",
        }
    }
}

impl Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Kernel Loader Error: {}", Error::description(self))
    }
}

#[derive(Debug, Default, Copy, Clone, PartialEq)]
/// Result of the KernelLoader load() call.
///
/// This specifies where the kernel is loading and passes additional
/// information for the rest of the boot process to be completed by
/// the VMM.
pub struct KernelLoaderResult {
    /// Address in the guest memory where the kernel image starts to be loaded
    pub kernel_load: GuestAddress,
    /// Offset in guest memory corresponding to the end of kernel image, in case that
    /// device tree blob and initrd will be loaded adjacent to kernel image.
    pub kernel_end: GuestUsize,
    /// This field is only for bzImage following https://www.kernel.org/doc/Documentation/x86/boot.txt
    /// VMM should make use of it to fill zero page for bzImage direct boot.
    #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
    pub setup_header: Option<bootparam::setup_header>,
    /// This field optionally holds the address of a PVH entry point, indicating that
    /// the kernel supports the PVH boot protocol as described in:
    /// https://xenbits.xen.org/docs/unstable/misc/pvh.html
    pub pvh_entry_addr: Option<GuestAddress>,
}

/// A kernel image loading support must implement the KernelLoader trait.
/// The only method to be implemented is the load one, returning a KernelLoaderResult structure.
pub trait KernelLoader {
    /// How to load a specific kernel image format into the guest memory.
    fn load<F, M: GuestMemory>(
        guest_mem: &M,
        kernel_start: Option<GuestAddress>,
        kernel_image: &mut F,
        highmem_start_address: Option<GuestAddress>,
    ) -> Result<KernelLoaderResult>
    where
        F: Read + Seek;
}

#[cfg(feature = "elf")]
#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
/// Raw ELF (a.k.a. vmlinux) kernel image support.
pub struct Elf;

#[cfg(feature = "elf")]
#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
impl KernelLoader for Elf {
    /// Loads a kernel from a vmlinux elf image to a slice
    ///
    /// kernel is loaded into guest memory at offset phdr.p_paddr specified by elf image.
    ///
    /// # Arguments
    ///
    /// * `guest_mem` - The guest memory region the kernel is written to.
    /// * `kernel_start` - The offset into 'guest_mem' at which to load the kernel.
    /// * `kernel_image` - Input vmlinux image.
    /// * `highmem_start_address` - This is the start of the high memory, kernel should above it.
    ///
    /// # Returns
    /// * KernelLoaderResult
    fn load<F, M: GuestMemory>(
        guest_mem: &M,
        kernel_start: Option<GuestAddress>,
        kernel_image: &mut F,
        highmem_start_address: Option<GuestAddress>,
    ) -> Result<KernelLoaderResult>
    where
        F: Read + Seek,
    {
        let mut ehdr: elf::Elf64_Ehdr = Default::default();
        kernel_image
            .seek(SeekFrom::Start(0))
            .map_err(|_| Error::SeekElfStart)?;
        unsafe {
            // read_struct is safe when reading a POD struct.  It can be used and dropped without issue.
            struct_util::read_struct(kernel_image, &mut ehdr).map_err(|_| Error::ReadElfHeader)?;
        }

        // Sanity checks
        if ehdr.e_ident[elf::EI_MAG0 as usize] != elf::ELFMAG0 as u8
            || ehdr.e_ident[elf::EI_MAG1 as usize] != elf::ELFMAG1
            || ehdr.e_ident[elf::EI_MAG2 as usize] != elf::ELFMAG2
            || ehdr.e_ident[elf::EI_MAG3 as usize] != elf::ELFMAG3
        {
            return Err(Error::InvalidElfMagicNumber);
        }
        if ehdr.e_ident[elf::EI_DATA as usize] != elf::ELFDATA2LSB as u8 {
            return Err(Error::BigEndianElfOnLittle);
        }
        if ehdr.e_phentsize as usize != mem::size_of::<elf::Elf64_Phdr>() {
            return Err(Error::InvalidProgramHeaderSize);
        }
        if (ehdr.e_phoff as usize) < mem::size_of::<elf::Elf64_Ehdr>() {
            return Err(Error::InvalidProgramHeaderOffset);
        }
        if (highmem_start_address.is_some())
            && ((ehdr.e_entry as u64) < highmem_start_address.unwrap().raw_value())
        {
            return Err(Error::InvalidEntryAddress);
        }

        let mut loader_result: KernelLoaderResult = Default::default();
        // where the kernel will be start loaded.
        loader_result.kernel_load = match kernel_start {
            Some(start) => GuestAddress(start.raw_value() + (ehdr.e_entry as u64)),
            None => GuestAddress(ehdr.e_entry as u64),
        };

        kernel_image
            .seek(SeekFrom::Start(ehdr.e_phoff))
            .map_err(|_| Error::SeekProgramHeader)?;
        let phdrs: Vec<elf::Elf64_Phdr> = unsafe {
            // Reading the structs is safe for a slice of POD structs.
            struct_util::read_struct_slice(kernel_image, ehdr.e_phnum as usize)
                .map_err(|_| Error::ReadProgramHeader)?
        };

        // Read in each section pointed to by the program headers.
        for phdr in &phdrs {
            if phdr.p_type != elf::PT_LOAD || phdr.p_filesz == 0 {
                if phdr.p_type == elf::PT_NOTE {
                    // This segment describes a Note, check if PVH entry point is encoded.
                    loader_result.pvh_entry_addr = parse_elf_note(phdr, kernel_image)?;
                }
                continue;
            }

            kernel_image
                .seek(SeekFrom::Start(phdr.p_offset))
                .map_err(|_| Error::SeekKernelStart)?;

            // if the vmm does not specify where the kernel should be loaded, just
            // load it to the physical address p_paddr for each segment.
            let mem_offset = match kernel_start {
                Some(start) => start
                    .checked_add(phdr.p_paddr as u64)
                    .ok_or(Error::InvalidProgramHeaderAddress)?,
                None => GuestAddress(phdr.p_paddr as u64),
            };

            guest_mem
                .read_exact_from(mem_offset, kernel_image, phdr.p_filesz as usize)
                .map_err(|_| Error::ReadKernelImage)?;

            loader_result.kernel_end = mem_offset
                .raw_value()
                .checked_add(phdr.p_memsz as GuestUsize)
                .ok_or(Error::MemoryOverflow)?;
        }

        // elf image has no setup_header which is defined for bzImage
        loader_result.setup_header = None;

        Ok(loader_result)
    }
}

#[cfg(feature = "elf")]
#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
fn parse_elf_note<F>(phdr: &elf::Elf64_Phdr, kernel_image: &mut F) -> Result<Option<GuestAddress>>
where
    F: Read + Seek,
{
    // Type of note header that encodes a 32-bit entry point address
    // to boot a guest kernel using the PVH boot protocol.
    const XEN_ELFNOTE_PHYS32_ENTRY: u32 = 18;

    let n_align = phdr.p_align;

    // Seek to the beginning of the note segment
    kernel_image
        .seek(SeekFrom::Start(phdr.p_offset))
        .map_err(|_| Error::SeekNoteHeader)?;

    // Now that the segment has been found, we must locate an ELF note with the
    // correct type that encodes the PVH entry point if there is one.
    let mut nhdr: elf::Elf64_Nhdr = Default::default();
    let mut read_size: usize = 0;

    while read_size < phdr.p_filesz as usize {
        unsafe {
            // read_struct is safe when reading a POD struct.
            // It can be used and dropped without issue.
            struct_util::read_struct(kernel_image, &mut nhdr).map_err(|_| Error::ReadNoteHeader)?;
        }
        // If the note header found is not the desired one, keep reading until
        // the end of the segment
        if nhdr.n_type == XEN_ELFNOTE_PHYS32_ENTRY {
            break;
        }
        // Skip the note header plus the size of its fields (with alignment)
        read_size += mem::size_of::<elf::Elf64_Nhdr>()
            + align_up(u64::from(nhdr.n_namesz), n_align)
            + align_up(u64::from(nhdr.n_descsz), n_align);

        kernel_image
            .seek(SeekFrom::Start(phdr.p_offset + read_size as u64))
            .map_err(|_| Error::SeekNoteHeader)?;
    }

    if read_size >= phdr.p_filesz as usize {
        return Ok(None); // PVH ELF note not found, nothing else to do.
    }
    // Otherwise the correct note type was found.
    // The note header struct has already been read, so we can seek from the
    // current position and just skip the name field contents.
    kernel_image
        .seek(SeekFrom::Current(
            align_up(u64::from(nhdr.n_namesz), n_align) as i64,
        ))
        .map_err(|_| Error::SeekNoteHeader)?;

    // The PVH entry point is a 32-bit address, so the descriptor field
    // must be capable of storing all such addresses.
    if (nhdr.n_descsz as usize) < mem::size_of::<u32>() {
        return Err(Error::InvalidPvhNote);
    }

    let mut pvh_addr_bytes = [0; mem::size_of::<u32>()];

    // Read 32-bit address stored in the PVH note descriptor field.
    kernel_image
        .read_exact(&mut pvh_addr_bytes)
        .map_err(|_| Error::ReadNoteHeader)?;

    Ok(Some(GuestAddress(
        u32::from_le_bytes(pvh_addr_bytes).into(),
    )))
}

#[cfg(feature = "bzimage")]
#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
/// Big zImage (bzImage) kernel image support.
pub struct BzImage;

#[cfg(feature = "bzimage")]
#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
impl KernelLoader for BzImage {
    /// Loads a bzImage
    ///
    /// kernel is loaded into guest memory at code32_start the default load address
    /// stored in bzImage setup header.
    ///
    /// # Arguments
    ///
    /// * `guest_mem` - The guest memory where the kernel image is loaded.
    /// * `kernel_start` - The offset into 'guest_mem' at which to load the kernel.
    /// * `kernel_image` - Input bzImage image.
    /// * `highmem_start_address` - This is the start of the high memory, kernel should above it.
    ///
    /// # Returns
    /// * KernelLoaderResult
    fn load<F, M: GuestMemory>(
        guest_mem: &M,
        kernel_start: Option<GuestAddress>,
        kernel_image: &mut F,
        highmem_start_address: Option<GuestAddress>,
    ) -> Result<KernelLoaderResult>
    where
        F: Read + Seek,
    {
        let mut kernel_size = kernel_image
            .seek(SeekFrom::End(0))
            .map_err(|_| Error::SeekBzImageEnd)? as usize;
        let mut boot_header: bootparam::setup_header = Default::default();
        kernel_image
            .seek(SeekFrom::Start(0x1F1))
            .map_err(|_| Error::SeekBzImageHeader)?;
        unsafe {
            // read_struct is safe when reading a POD struct.  It can be used and dropped without issue.
            struct_util::read_struct(kernel_image, &mut boot_header)
                .map_err(|_| Error::ReadBzImageHeader)?;
        }

        // if the HdrS magic number is not found at offset 0x202, the boot protocol version is "old",
        // the image type is assumed as zImage, not bzImage.
        if boot_header.header != 0x5372_6448 {
            return Err(Error::InvalidBzImage);
        }

        // follow section of loading the rest of the kernel in linux boot protocol
        if (boot_header.version < 0x0200) || ((boot_header.loadflags & 0x1) == 0x0) {
            return Err(Error::InvalidBzImage);
        }

        let mut setup_size = boot_header.setup_sects as usize;
        if setup_size == 0 {
            setup_size = 4;
        }
        setup_size = (setup_size + 1) * 512;
        kernel_size -= setup_size;

        // verify bzImage validation by checking if code32_start, the defaults to the address of
        // the kernel is not lower than high memory.
        if (highmem_start_address.is_some())
            && (u64::from(boot_header.code32_start) < highmem_start_address.unwrap().raw_value())
        {
            return Err(Error::InvalidKernelStartAddress);
        }

        let mem_offset = match kernel_start {
            Some(start) => start,
            None => GuestAddress(u64::from(boot_header.code32_start)),
        };

        boot_header.code32_start = mem_offset.raw_value() as u32;

        let mut loader_result: KernelLoaderResult = Default::default();
        loader_result.setup_header = Some(boot_header);
        loader_result.kernel_load = mem_offset;

        //seek the compressed vmlinux.bin and read to memory
        kernel_image
            .seek(SeekFrom::Start(setup_size as u64))
            .map_err(|_| Error::SeekBzImageCompressedKernel)?;
        guest_mem
            .read_exact_from(mem_offset, kernel_image, kernel_size)
            .map_err(|_| Error::ReadBzImageCompressedKernel)?;

        loader_result.kernel_end = mem_offset
            .raw_value()
            .checked_add(kernel_size as GuestUsize)
            .ok_or(Error::MemoryOverflow)?;

        Ok(loader_result)
    }
}

/// Writes the command line string to the given memory slice.
///
/// # Arguments
///
/// * `guest_mem` - A u8 slice that will be partially overwritten by the command line.
/// * `guest_addr` - The address in `guest_mem` at which to load the command line.
/// * `cmdline` - The kernel command line.
pub fn load_cmdline<M: GuestMemory>(
    guest_mem: &M,
    guest_addr: GuestAddress,
    cmdline: &CStr,
) -> Result<()> {
    let len = cmdline.to_bytes().len();
    if len == 0 {
        return Ok(());
    }

    let end = guest_addr
        .checked_add(len as u64 + 1)
        .ok_or(Error::CommandLineOverflow)?; // Extra for null termination.
    if end > guest_mem.last_addr() {
        return Err(Error::CommandLineOverflow);
    }

    guest_mem
        .write_slice(cmdline.to_bytes_with_nul(), guest_addr)
        .map_err(|_| Error::CommandLineCopy)?;

    Ok(())
}

/// Align address upwards. Taken from x86_64 crate:
/// https://docs.rs/x86_64/latest/x86_64/fn.align_up.html
///
/// Returns the smallest x with alignment `align` so that x >= addr. The alignment must be
/// a power of 2.
#[cfg(feature = "elf")]
#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
fn align_up(addr: u64, align: u64) -> usize {
    assert!(align.is_power_of_two(), "`align` must be a power of two");
    let align_mask = align - 1;
    if addr & align_mask == 0 {
        addr as usize // already aligned
    } else {
        ((addr | align_mask) + 1) as usize
    }
}

/// Writes the device tree to the given memory slice.
///
/// # Arguments
///
/// * `guest_mem` - A u8 slice that will be partially overwritten by the device tree blob.
/// * `guest_addr` - The address in `guest_mem` at which to load the device tree blob.
/// * `dtb_image` - The device tree blob.
#[cfg(target_arch = "aarch64")]
pub fn load_dtb<F, M: GuestMemory>(
    guest_mem: &M,
    guest_addr: GuestAddress,
    dtb_image: &mut F,
) -> Result<()>
where
    F: Read + Seek,
{
    let dtb_size = dtb_image
        .seek(SeekFrom::End(0))
        .map_err(|_| Error::SeekDtbEnd)? as usize;
    if dtb_size > 0x200000 {
        return Err(Error::DtbTooBig);
    }
    dtb_image
        .seek(SeekFrom::Start(0))
        .map_err(|_| Error::SeekDtbStart)?;
    guest_mem
        .read_exact_from(guest_addr, dtb_image, dtb_size)
        .map_err(|_| Error::ReadDtbImage)
}

#[cfg(feature = "pe")]
#[cfg(target_arch = "aarch64")]
/// ARM64 Image (PE) format support
pub struct PE;

#[cfg(feature = "pe")]
#[cfg(target_arch = "aarch64")]
#[repr(C)]
#[derive(Debug, Copy, Clone, Default)]
// See kernel doc Documentation/arm64/booting.txt for more information.
// All these fields should be little endian.
struct arm64_image_header {
    code0: u32,
    code1: u32,
    text_offset: u64,
    image_size: u64,
    flags: u64,
    res2: u64,
    res3: u64,
    res4: u64,
    magic: u32,
    res5: u32,
}

#[cfg(feature = "pe")]
#[cfg(target_arch = "aarch64")]
impl KernelLoader for PE {
    /// Loads a PE Image into guest memory.
    ///
    /// # Arguments
    ///
    /// * `guest_mem` - The guest memory where the kernel image is loaded.
    /// * `kernel_start` - The offset into 'guest_mem' at which to load the kernel.
    /// * `kernel_image` - Input Image image.
    /// * `highmem_start_address` - ignored on ARM64
    ///
    /// # Returns
    /// * KernelLoaderResult
    fn load<F, M: GuestMemory>(
        guest_mem: &M,
        kernel_start: Option<GuestAddress>,
        kernel_image: &mut F,
        _highmem_start_address: Option<GuestAddress>,
    ) -> Result<KernelLoaderResult>
    where
        F: Read + Seek,
    {
        let kernel_size = kernel_image
            .seek(SeekFrom::End(0))
            .map_err(|_| Error::SeekImageEnd)? as usize;
        let mut arm64_header: arm64_image_header = Default::default();
        kernel_image
            .seek(SeekFrom::Start(0))
            .map_err(|_| Error::SeekImageHeader)?;
        unsafe {
            // read_struct is safe when reading a POD struct.  It can be used and dropped without issue.
            struct_util::read_struct(kernel_image, &mut arm64_header)
                .map_err(|_| Error::ReadImageHeader)?;
        }

        if u32::from_le(arm64_header.magic) != 0x644d_5241 {
            return Err(Error::InvalidImageMagicNumber);
        }

        let image_size = u64::from_le(arm64_header.image_size);
        let mut text_offset = u64::from_le(arm64_header.text_offset);

        if image_size == 0 {
            text_offset = 0x80000;
        }

        let mem_offset = kernel_start
            .unwrap_or(GuestAddress(0))
            .checked_add(text_offset)
            .ok_or(Error::InvalidImage)?;

        let mut loader_result: KernelLoaderResult = Default::default();
        loader_result.kernel_load = mem_offset;

        kernel_image
            .seek(SeekFrom::Start(0))
            .map_err(|_| Error::SeekImageHeader)?;
        guest_mem
            .read_exact_from(mem_offset, kernel_image, kernel_size)
            .map_err(|_| Error::ReadKernelImage)?;

        loader_result.kernel_end = mem_offset
            .raw_value()
            .checked_add(kernel_size as GuestUsize)
            .ok_or(Error::MemoryOverflow)?;

        Ok(loader_result)
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use std::io::Cursor;
    use vm_memory::{Address, GuestAddress, GuestMemoryMmap};

    const MEM_SIZE: u64 = 0x1000000;

    fn create_guest_mem() -> GuestMemoryMmap {
        GuestMemoryMmap::from_ranges(&[(GuestAddress(0x0), (MEM_SIZE as usize))]).unwrap()
    }

    #[cfg(feature = "bzimage")]
    #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
    fn make_bzimage() -> Vec<u8> {
        let mut v = Vec::new();
        v.extend_from_slice(include_bytes!("bzimage"));
        v
    }

    // Elf64 image that prints hello world on x86_64.
    #[cfg(feature = "elf")]
    #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
    fn make_elf_bin() -> Vec<u8> {
        let mut v = Vec::new();
        v.extend_from_slice(include_bytes!("test_elf.bin"));
        v
    }

    #[cfg(feature = "elf")]
    #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
    fn make_elfnote() -> Vec<u8> {
        include_bytes!("test_elfnote.bin").to_vec()
    }

    #[cfg(feature = "elf")]
    #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
    fn make_dummy_elfnote() -> Vec<u8> {
        include_bytes!("test_dummynote.bin").to_vec()
    }

    #[cfg(feature = "elf")]
    #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
    fn make_bad_elfnote() -> Vec<u8> {
        include_bytes!("test_badnote.bin").to_vec()
    }

    // first 4096 bytes of a precompiled ARM64 kernel Image.
    #[cfg(feature = "pe")]
    #[cfg(target_arch = "aarch64")]
    fn make_image_bin() -> Vec<u8> {
        let mut v = Vec::new();
        v.extend_from_slice(include_bytes!("test_image.bin"));
        v
    }

    #[allow(safe_packed_borrows)]
    #[allow(non_snake_case)]
    #[test]
    #[cfg(feature = "bzimage")]
    #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
    fn load_bzImage() {
        let gm = create_guest_mem();
        let image = make_bzimage();
        let mut kernel_start = GuestAddress(0x200000);
        let mut highmem_start_address = GuestAddress(0x0);

        // load bzImage with good kernel_start and himem_start setting
        let mut loader_result = BzImage::load(
            &gm,
            Some(kernel_start),
            &mut Cursor::new(&image),
            Some(highmem_start_address),
        )
        .unwrap();
        assert_eq!(0x53726448, loader_result.setup_header.unwrap().header);
        println!(
            "bzImage is loaded at {:8x} \n",
            loader_result.kernel_load.raw_value()
        );
        println!(
            "bzImage version is {:2x} \n",
            loader_result.setup_header.unwrap().version
        );
        println!(
            "bzImage loadflags is {:x} \n",
            loader_result.setup_header.unwrap().loadflags
        );
        println!(
            "bzImage kernel size is {:4x} \n",
            (loader_result.kernel_end as u32)
        );

        // load bzImage without kernel_start
        loader_result = BzImage::load(
            &gm,
            None,
            &mut Cursor::new(&image),
            Some(highmem_start_address),
        )
        .unwrap();
        assert_eq!(0x53726448, loader_result.setup_header.unwrap().header);
        println!(
            "bzImage is loaded at {:8x} \n",
            loader_result.kernel_load.raw_value()
        );

        // load bzImage withouth himem_start
        loader_result = BzImage::load(&gm, None, &mut Cursor::new(&image), None).unwrap();
        assert_eq!(0x53726448, loader_result.setup_header.unwrap().header);
        println!(
            "bzImage is loaded at {:8x} \n",
            loader_result.kernel_load.raw_value()
        );

        // load bzImage with a bad himem setting
        kernel_start = GuestAddress(0x1000);
        highmem_start_address = GuestAddress(0x200000);
        let x = BzImage::load(
            &gm,
            Some(kernel_start),
            &mut Cursor::new(&image),
            Some(highmem_start_address),
        );
        assert_eq!(x.is_ok(), false);
        println!("load bzImage with bad himem setting \n");
    }

    #[test]
    #[cfg(feature = "elf")]
    #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
    fn load_elf() {
        let gm = create_guest_mem();
        let image = make_elf_bin();
        let kernel_addr = GuestAddress(0x200000);
        let mut highmem_start_address = GuestAddress(0x0);
        let mut loader_result = Elf::load(
            &gm,
            Some(kernel_addr),
            &mut Cursor::new(&image),
            Some(highmem_start_address),
        )
        .unwrap();
        println!(
            "load elf at address {:8x} \n",
            loader_result.kernel_load.raw_value()
        );

        loader_result = Elf::load(&gm, Some(kernel_addr), &mut Cursor::new(&image), None).unwrap();
        println!(
            "load elf at address {:8x} \n",
            loader_result.kernel_load.raw_value()
        );

        loader_result = Elf::load(
            &gm,
            None,
            &mut Cursor::new(&image),
            Some(highmem_start_address),
        )
        .unwrap();
        println!(
            "load elf at address {:8x} \n",
            loader_result.kernel_load.raw_value()
        );

        highmem_start_address = GuestAddress(0xa00000);
        assert_eq!(
            Err(Error::InvalidEntryAddress),
            Elf::load(
                &gm,
                None,
                &mut Cursor::new(&image),
                Some(highmem_start_address)
            )
        );
    }

    #[cfg(feature = "elf")]
    #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
    #[test]
    fn load_pvh() {
        let gm = create_guest_mem();
        let pvhnote_image = make_elfnote();
        let loader_result = Elf::load(&gm, None, &mut Cursor::new(&pvhnote_image), None).unwrap();
        println!(
            "PVH entry point at address {:8x} \n",
            loader_result.pvh_entry_addr.unwrap().raw_value()
        );
        assert_eq!(loader_result.pvh_entry_addr.unwrap().raw_value(), 0x1e1fe1f);
    }

    #[test]
    #[cfg(feature = "elf")]
    #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
    fn dumy_elfnote() {
        let gm = create_guest_mem();
        let dummynote_image = make_dummy_elfnote();
        let loader_result = Elf::load(&gm, None, &mut Cursor::new(&dummynote_image), None).unwrap();
        assert!(loader_result.pvh_entry_addr.is_none());
    }

    #[test]
    #[cfg(feature = "elf")]
    #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
    fn bad_elfnote() {
        let gm = create_guest_mem();
        let badnote_image = make_bad_elfnote();
        assert_eq!(
            Err(Error::InvalidPvhNote),
            Elf::load(&gm, None, &mut Cursor::new(&badnote_image), None)
        );
    }

    #[test]
    #[cfg(feature = "pe")]
    #[cfg(target_arch = "aarch64")]
    fn load_image() {
        let gm = create_guest_mem();
        let mut image = make_image_bin();
        let kernel_addr = GuestAddress(0x200000);

        let loader_result =
            PE::load(&gm, Some(kernel_addr), &mut Cursor::new(&image), None).unwrap();
        assert_eq!(loader_result.kernel_load.raw_value(), 0x280000);
        assert_eq!(loader_result.kernel_end, 0x281000);

        image[0x39] = 0x0;
        let loader_result = PE::load(&gm, Some(kernel_addr), &mut Cursor::new(&image), None);
        assert_eq!(loader_result, Err(Error::InvalidImageMagicNumber));
    }

    #[test]
    fn cmdline_overflow() {
        let gm = create_guest_mem();
        let cmdline_address = GuestAddress(MEM_SIZE - 5);
        assert_eq!(
            Err(Error::CommandLineOverflow),
            load_cmdline(
                &gm,
                cmdline_address,
                CStr::from_bytes_with_nul(b"12345\0").unwrap()
            )
        );
    }

    #[test]
    fn cmdline_write_end() {
        let gm = create_guest_mem();
        let mut cmdline_address = GuestAddress(45);
        assert_eq!(
            Ok(()),
            load_cmdline(
                &gm,
                cmdline_address,
                CStr::from_bytes_with_nul(b"1234\0").unwrap()
            )
        );
        let val: u8 = gm.read_obj(cmdline_address).unwrap();
        assert_eq!(val, '1' as u8);
        cmdline_address = cmdline_address.unchecked_add(1);
        let val: u8 = gm.read_obj(cmdline_address).unwrap();
        assert_eq!(val, '2' as u8);
        cmdline_address = cmdline_address.unchecked_add(1);
        let val: u8 = gm.read_obj(cmdline_address).unwrap();
        assert_eq!(val, '3' as u8);
        cmdline_address = cmdline_address.unchecked_add(1);
        let val: u8 = gm.read_obj(cmdline_address).unwrap();
        assert_eq!(val, '4' as u8);
        cmdline_address = cmdline_address.unchecked_add(1);
        let val: u8 = gm.read_obj(cmdline_address).unwrap();
        assert_eq!(val, '\0' as u8);
    }

    #[cfg(feature = "elf")]
    #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
    #[test]
    fn bad_magic() {
        let gm = create_guest_mem();
        let kernel_addr = GuestAddress(0x0);
        let mut bad_image = make_elf_bin();
        bad_image[0x1] = 0x33;
        assert_eq!(
            Err(Error::InvalidElfMagicNumber),
            Elf::load(&gm, Some(kernel_addr), &mut Cursor::new(&bad_image), None)
        );
    }

    #[cfg(feature = "elf")]
    #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
    #[test]
    fn bad_endian() {
        // Only little endian is supported
        let gm = create_guest_mem();
        let kernel_addr = GuestAddress(0x0);
        let mut bad_image = make_elf_bin();
        bad_image[0x5] = 2;
        assert_eq!(
            Err(Error::BigEndianElfOnLittle),
            Elf::load(&gm, Some(kernel_addr), &mut Cursor::new(&bad_image), None)
        );
    }

    #[cfg(feature = "elf")]
    #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
    #[test]
    fn bad_phoff() {
        // program header has to be past the end of the elf header
        let gm = create_guest_mem();
        let kernel_addr = GuestAddress(0x0);
        let mut bad_image = make_elf_bin();
        bad_image[0x20] = 0x10;
        assert_eq!(
            Err(Error::InvalidProgramHeaderOffset),
            Elf::load(&gm, Some(kernel_addr), &mut Cursor::new(&bad_image), None)
        );
    }
}
