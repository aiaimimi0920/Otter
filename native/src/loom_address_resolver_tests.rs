use super::*;

#[test]
fn numeric_addresses_and_invalid_inputs_do_not_need_dns() {
    let cancel = Cancel::new();
    assert_eq!(
        resolve("127.0.0.1", 443, &cancel).unwrap(),
        vec!["127.0.0.1:443".parse().unwrap()]
    );
    assert_eq!(
        resolve("::1", 8443, &cancel).unwrap(),
        vec!["[::1]:8443".parse().unwrap()]
    );
    for host in [
        "",
        "https://localhost",
        "localhost:443",
        "a\0b",
        "a b",
        "[::1]",
    ] {
        assert_eq!(resolve(host, 443, &cancel), Err(Error::Input));
    }
    assert_eq!(resolve("localhost", 0, &cancel), Err(Error::Input));
    cancel.cancel();
    assert_eq!(resolve("localhost", 443, &cancel), Err(Error::Stopped));
}

#[test]
fn actual_windows_localhost_resolution_returns_bounded_loopback_addresses() {
    let addresses = resolve("localhost", 8443, &Cancel::new()).unwrap();
    assert!(!addresses.is_empty() && addresses.len() <= 4);
    assert!(addresses
        .iter()
        .all(|a| a.ip().is_loopback() && a.port() == 8443));
}

#[test]
fn native_cancel_event_covers_install_before_and_after_cancel() {
    for before in [false, true] {
        let cancel = Cancel::new();
        let event = Arc::new(event().unwrap());
        if before {
            cancel.cancel();
        }
        cancel.install_wake(event.clone());
        if !before {
            cancel.cancel();
        }
        assert_eq!(
            unsafe { WaitForSingleObject(HANDLE(event.as_raw_handle()), 1000) },
            WAIT_OBJECT_0
        );
        cancel.clear_wake();
    }
}

#[test]
fn sockaddr_conversion_preserves_network_byte_order_and_ipv6_scope() {
    let mut ipv4 = SOCKADDR_IN {
        sin_family: AF_INET,
        sin_port: 8443u16.to_be(),
        ..Default::default()
    };
    ipv4.sin_addr.S_un.S_addr = u32::from_ne_bytes([192, 0, 2, 9]);
    let mut ipv6 = SOCKADDR_IN6 {
        sin6_family: AF_INET6,
        sin6_port: 8443u16.to_be(),
        sin6_flowinfo: 123u32.to_be(),
        ..Default::default()
    };
    ipv6.sin6_addr.u.Byte = "fe80::1".parse::<Ipv6Addr>().unwrap().octets();
    ipv6.Anonymous.sin6_scope_id = 7;
    let mut second = ADDRINFOEXW {
        ai_family: AF_INET6.0 as i32,
        ai_addrlen: std::mem::size_of::<SOCKADDR_IN6>(),
        ai_addr: std::ptr::addr_of_mut!(ipv6).cast(),
        ..Default::default()
    };
    let first = ADDRINFOEXW {
        ai_family: AF_INET.0 as i32,
        ai_addrlen: std::mem::size_of::<SOCKADDR_IN>(),
        ai_addr: std::ptr::addr_of_mut!(ipv4).cast(),
        ai_next: &mut second,
        ..Default::default()
    };
    assert_eq!(
        unsafe { collect(&first, 8443) }.unwrap(),
        vec![
            "192.0.2.9:8443".parse().unwrap(),
            SocketAddr::V6(SocketAddrV6::new("fe80::1".parse().unwrap(), 8443, 123, 7))
        ]
    );
}

#[test]
fn malformed_address_lengths_and_cyclic_result_lists_are_bounded() {
    let mut address = SOCKADDR_IN::default();
    let mut info = ADDRINFOEXW {
        ai_family: AF_INET.0 as i32,
        ai_addrlen: 1,
        ai_addr: std::ptr::addr_of_mut!(address).cast(),
        ..Default::default()
    };
    assert_eq!(unsafe { collect(&info, 443) }, Err(Error::Empty));
    info.ai_next = &mut info;
    assert_eq!(unsafe { collect(&info, 443) }, Err(Error::Limit));
}
