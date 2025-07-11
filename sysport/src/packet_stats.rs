use egui::Color32;

#[derive(Default)]
pub struct PacketStats {
    pub tcp_count: usize,
    pub udp_count: usize,
    pub icmp_count: usize,
    pub arp_count: usize,
    pub tcp_bytes: usize,
    pub udp_bytes: usize,
    pub icmp_bytes: usize,
    pub arp_bytes: usize,
}

impl PacketStats {
    pub fn update(&mut self, pkt: &[u8]) {
        let (proto, _color) = decode_protocol(pkt);
        match proto.as_str() {
            "TCP" => {
                self.tcp_count += 1;
                self.tcp_bytes += pkt.len();
            }
            "UDP" => {
                self.udp_count += 1;
                self.udp_bytes += pkt.len();
            }
            "ICMP" => {
                self.icmp_count += 1;
                self.icmp_bytes += pkt.len();
            }
            "ARP" => {
                self.arp_count += 1;
                self.arp_bytes += pkt.len();
            }
            _ => {}
        }
    }
    pub fn reset(&mut self) {
        *self = Self::default();
    }
}

pub fn decode_protocol(pkt: &[u8]) -> (String, Color32) {
    if pkt.len() > 20 && (pkt[0] >> 4) == 4 {
        // IPv4
        let proto = pkt[9];
        if proto == 6 {
            ("TCP".to_string(), Color32::LIGHT_BLUE)
        } else if proto == 17 {
            ("UDP".to_string(), Color32::LIGHT_GREEN)
        } else if proto == 1 {
            ("ICMP".to_string(), Color32::YELLOW)
        } else {
            ("IPv4".to_string(), Color32::GRAY)
        }
    } else if pkt.len() > 14 && pkt[12] == 0x08 && pkt[13] == 0x06 {
        ("ARP".to_string(), Color32::RED)
    } else {
        ("Other".to_string(), Color32::GRAY)
    }
} 