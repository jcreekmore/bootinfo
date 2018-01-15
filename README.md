bootinfo
========

A tool for displaying boot information out of a binary file. For example, it will parse and display
the Multiboot and Multiboot2 headers from a Multiboot-compliant binary.

Quick Start
-----------

To install:

```
$ cargo install bootinfo
```

To run on a binary:

```
$ bootinfo /boot/xen.gz
Multiboot Header
  Magic     : 0x1badb002
  Flags     : [page-aligned-modules, request-memory-map] (0x00000003)
  Checksum  : 0xe4524ffb

Multiboot2 Header
  Magic       : 0xe85250d6
  Arch        : 0x00000000
  Header Len  : 0x00000088
  Checksum    : 0x17adaea2
  Tag: Information Request (1)
    Flags      : [required] (0x0000)
    Size       : 16 bytes
    Types      : [4, 6]
  Tag: Module Alignment (6)
    Flags      : [required] (0x0000)
    Size       : 8 bytes
  Tag: Relocatable (10)
    Flags      : [optional] (0x0001)
    Size       : 24 bytes
    Min Addr   : 0x200000
    Max Addr   : 0xffffffff
    Align      : 0x200000
    Preference : maximum
  Tag: Flags (4)
    Flags      : [optional] (0x0001)
    Size       : 12 bytes
    Console    : 0x2
  Tag: Framebuffer (5)
    Flags      : [optional] (0x0001)
    Size       : 20 bytes
    Width      : 0
    Height     : 0
    Depth      : 0
  Tag: EFI Boot Services (7)
    Flags      : [optional] (0x0001)
    Size       : 8 bytes
  Tag: EFI amd64 Entry (9)
    Flags      : [optional] (0x0001)
    Size       : 12 bytes
    Entry      : 0x38405d
```
