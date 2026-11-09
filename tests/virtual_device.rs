#![cfg(feature = "tokio")]

use std::error::Error;
use std::thread::sleep;
use std::time::Duration;

use tokio::time::timeout;

use evdev::uinput::{VirtualDevice, VirtualDeviceBuilder};
use evdev::{AttributeSet, EventType, InputEvent, KeyCode};

#[tokio::test]
async fn test_virtual_device_actually_emits() -> Result<(), Box<dyn Error>> {
    let mut keys = AttributeSet::<KeyCode>::new();
    let virtual_device_name = "fake-keyboard";
    keys.insert(KeyCode::KEY_ESC);

    let mut device = VirtualDevice::builder()?
        .name(virtual_device_name)
        .with_keys(&keys)?
        .build()
        .unwrap();

    let mut maybe_device = None;
    sleep(Duration::from_millis(500));
    for (_i, d) in evdev::enumerate() {
        println!("{:?}", d.name());
        if d.name() == Some(virtual_device_name) {
            maybe_device = Some(d);
            break;
        }
    }
    assert!(maybe_device.is_some());
    let listen_device = maybe_device.unwrap();

    let type_ = EventType::KEY;
    let code = KeyCode::KEY_ESC.code();

    // listen for events on the listen device
    let listener = tokio::spawn(async move {
        // try to read the key code that will be sent through virtual device
        let mut events = listen_device.into_event_stream()?;
        events.next_event().await
    });

    // emit a key code through virtual device
    let down_event = InputEvent::new(type_.0, code, 10);
    device.emit(&[down_event]).unwrap();

    let event = timeout(Duration::from_secs(1), listener).await???;

    assert_eq!(down_event.event_type(), event.event_type());
    assert_eq!(down_event.code(), event.code());

    // wait for listener
    Ok(())
}

/// A device built on a write-only fd (`VirtualDeviceBuilder::from_fd`) emits too.
#[tokio::test]
async fn test_virtual_device_from_write_only_fd_emits() -> Result<(), Box<dyn Error>> {
    let mut keys = AttributeSet::<KeyCode>::new();
    let virtual_device_name = "fake-keyboard-write-only";
    keys.insert(KeyCode::KEY_ESC);

    let fd = std::fs::OpenOptions::new()
        .write(true)
        .open("/dev/uinput")?;
    let mut device = VirtualDeviceBuilder::from_fd(fd.into())
        .name(virtual_device_name)
        .with_keys(&keys)?
        .build()?;

    sleep(Duration::from_millis(500));
    let listen_device = evdev::enumerate()
        .map(|(_, d)| d)
        .find(|d| d.name() == Some(virtual_device_name))
        .expect("the write-only device is listed");

    let code = KeyCode::KEY_ESC.code();
    let listener = tokio::spawn(async move {
        let mut events = listen_device.into_event_stream()?;
        events.next_event().await
    });
    let down_event = InputEvent::new(EventType::KEY.0, code, 10);
    device.emit(&[down_event])?;
    let event = timeout(Duration::from_secs(1), listener).await???;
    assert_eq!(down_event.code(), event.code());
    Ok(())
}
