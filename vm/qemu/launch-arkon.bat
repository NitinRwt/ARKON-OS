@echo off
REM launch-arkon.bat — Launch ARKON QEMU VM on Windows
REM
REM Requirements:
REM   - QEMU for Windows (from https://qemu.weilnetz.de/ or Chocolatey: choco install qemu)
REM   - OVMF firmware: included with QEMU for Windows in C:\Program Files\qemu\share\
REM   - The ARKON QCOW2 must already be built (run build on a Linux host first)
REM
REM IMPORTANT: This script cannot build ARKON images (Podman requires Linux).
REM            Copy a built QCOW2 from your Linux build host to artifacts\qcow2\
REM            before running this script.

setlocal

set QEMU_DIR=C:\Program Files\qemu
set QEMU_EXE=%QEMU_DIR%\qemu-system-x86_64.exe
set OVMF_CODE=%QEMU_DIR%\share\edk2-x86_64-code.fd
set QCOW2=%~dp0..\..\artifacts\qcow2\disk.qcow2

if not exist "%QEMU_EXE%" (
    echo ERROR: QEMU not found at %QEMU_EXE%
    echo        Install QEMU: https://qemu.weilnetz.de/
    exit /b 1
)

if not exist "%OVMF_CODE%" (
    echo ERROR: OVMF firmware not found at %OVMF_CODE%
    echo        OVMF should be included with QEMU for Windows.
    exit /b 1
)

if not exist "%QCOW2%" (
    echo ERROR: QCOW2 not found at %QCOW2%
    echo        Build ARKON on a Linux host, then copy the QCOW2 to artifacts\qcow2\
    exit /b 1
)

echo Booting ARKON OS in QEMU (Windows host, no KVM acceleration)
echo Image:  %QCOW2%
echo UEFI:   %OVMF_CODE%
echo vCPUs:  4   RAM: 8192 MB
echo.
echo NOTE: KVM is not available on Windows. Boot will be slower than on Linux.

"%QEMU_EXE%" ^
    -cpu max ^
    -smp 4 ^
    -m 8192 ^
    -machine type=q35 ^
    -drive if=pflash,format=raw,unit=0,file="%OVMF_CODE%",readonly=on ^
    -drive if=virtio,format=qcow2,file="%QCOW2%",cache=writeback ^
    -netdev user,id=net0 ^
    -device virtio-net-pci,netdev=net0 ^
    -vga virtio ^
    -display sdl ^
    -rtc base=utc

endlocal
