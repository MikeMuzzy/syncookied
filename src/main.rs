extern crate libc;
extern crate pnet;

use std::env;
use std::process;
use std::thread;

use std::net::IpAddr;

use pnet::packet::Packet;
use pnet::packet::ethernet::{EthernetPacket, EtherTypes};
use pnet::packet::ip::{IpNextHeaderProtocol, IpNextHeaderProtocols};
use pnet::packet::ipv4::Ipv4Packet;
use pnet::packet::tcp::TcpPacket;

mod netmap;
use netmap::{Action,NetmapDescriptor};

// helpers
fn get_cpu_count() -> usize {
    unsafe { 
        libc::sysconf(libc::_SC_NPROCESSORS_ONLN) as usize
    } 
}

fn handle_tcp_packet(source: IpAddr, destination: IpAddr, packet: &[u8]) {
    let tcp = TcpPacket::new(packet);
    if let Some(tcp) = tcp {
        //println!("TCP Packet: {}:{} > {}:{}; length: {}", source,
        //            tcp.get_source(), destination, tcp.get_destination(), packet.len());
        println!("TCP Packet: {:?}", tcp);
    } else {
        println!("Malformed TCP Packet");
    }
}

fn handle_transport_protocol(source: IpAddr, destination: IpAddr,
                             protocol: IpNextHeaderProtocol, packet: &[u8]) {
    match protocol {
        IpNextHeaderProtocols::Tcp  => handle_tcp_packet(source, destination, packet),
        _ => println!("Unknown {} packet: {} > {}; protocol: {:?} length: {}",
                match source { IpAddr::V4(..) => "IPv4", _ => "IPv6" },
                source,
                destination,
                protocol,
                packet.len())

    }
}

fn handle_ipv4_packet(ethernet: &EthernetPacket) {
    let header = Ipv4Packet::new(ethernet.payload());
    if let Some(header) = header {
        handle_transport_protocol(IpAddr::V4(header.get_source()),
                                  IpAddr::V4(header.get_destination()),
                                  header.get_next_level_protocol(),
                                  header.payload());
    } else {
        println!("Malformed IPv4 Packet");
    }
}

fn handle_ether_packet(ethernet: &EthernetPacket) {
    println!("{:?}", &ethernet);
    match ethernet.get_ethertype() {
        EtherTypes::Ipv4 => handle_ipv4_packet(ethernet),
        //EtherTypes::Ipv6 => handle_ipv6_packet(interface_name, ethernet),
        //EtherTypes::Arp  => handle_arp_packet(interface_name, ethernet),
        _                => println!("Unknown packet: {} > {}; ethertype: {:?} length: {}",
                                        ethernet.get_source(),
                                        ethernet.get_destination(),
                                        ethernet.get_ethertype(),
                                        ethernet.packet().len())
    }
}

fn handle_packet(packet_data: &[u8]) -> Action {
    let eth = EthernetPacket::new(packet_data).unwrap();
    handle_ether_packet(&eth);
    Action::Forward
}

fn rx_loop(netmap: &mut NetmapDescriptor) {
        println!("Rx rings: {:?}", netmap.get_rx_rings());
        println!("Tx rings: {:?}", netmap.get_tx_rings());
        thread::sleep_ms(1000);
        for _ in 0..100 {
            netmap.poll(handle_packet);
        }
}

fn run(iface: &str) {
    let nm = NetmapDescriptor::new(iface).unwrap();
    println!("Rx rings: {}, Tx rings: {} flags: {}", nm.get_rx_rings_count(), nm.get_tx_rings_count(), nm.get_flags());

    for ring in 0..nm.get_rx_rings_count() {
        let mut ring_nm = nm.clone_ring(ring).unwrap();
        /* XXX */
        rx_loop(&mut ring_nm);
    }
}

fn main() {
    let iface = env::args().nth(1).unwrap();
    let ncpus = get_cpu_count();
    println!("interface: {} cores: {}", iface, ncpus);
    run(&iface);
}