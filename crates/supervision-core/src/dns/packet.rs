#[derive(Debug, Clone)]
pub struct DnsQuestion {
    pub name: String,
    pub qtype: u16,
    pub qclass: u16,
}

#[derive(Debug, Clone)]
pub struct DnsMessage {
    pub id: u16,
    pub flags: u16,
    pub questions: Vec<DnsQuestion>,
    pub raw: Vec<u8>,
}

impl DnsMessage {
    pub fn parse(buf: &[u8]) -> Option<Self> {
        if buf.len() < 12 {
            return None;
        }

        let id = u16::from_be_bytes([buf[0], buf[1]]);
        let flags = u16::from_be_bytes([buf[2], buf[3]]);
        let qdcount = u16::from_be_bytes([buf[4], buf[5]]) as usize;

        let mut pos = 12;
        let mut questions = Vec::with_capacity(qdcount);

        for _ in 0..qdcount {
            let (name, next_pos) = Self::parse_qname(buf, pos)?;
            pos = next_pos;

            if pos + 4 > buf.len() {
                return None;
            }
            let qtype = u16::from_be_bytes([buf[pos], buf[pos + 1]]);
            let qclass = u16::from_be_bytes([buf[pos + 2], buf[pos + 3]]);
            pos += 4;

            questions.push(DnsQuestion { name, qtype, qclass });
        }

        Some(Self {
            id,
            flags,
            questions,
            raw: buf.to_vec(),
        })
    }

    fn parse_qname(buf: &[u8], mut pos: usize) -> Option<(String, usize)> {
        let mut labels = Vec::new();
        let mut jumped = false;
        let mut final_pos = pos;

        let mut steps = 0;
        while steps < 100 {
            steps += 1;
            if pos >= buf.len() {
                return None;
            }

            let len = buf[pos] as usize;
            if len == 0 {
                if !jumped {
                    final_pos = pos + 1;
                }
                break;
            }

            if (len & 0xC0) == 0xC0 {
                if pos + 1 >= buf.len() {
                    return None;
                }
                let ptr = (((len & 0x3F) << 8) | (buf[pos + 1] as usize)) as usize;
                if !jumped {
                    final_pos = pos + 2;
                    jumped = true;
                }
                pos = ptr;
                continue;
            }

            pos += 1;
            if pos + len > buf.len() {
                return None;
            }
            let label = std::str::from_utf8(&buf[pos..pos + len]).ok()?.to_lowercase();
            labels.push(label);
            pos += len;
        }

        Some((labels.join("."), final_pos))
    }

    /// Synthesize an NXDOMAIN response (Domain Not Found)
    pub fn build_nxdomain_response(&self) -> Vec<u8> {
        let mut resp = Vec::with_capacity(self.raw.len() + 16);
        // Copy original query up to the end of the question section
        let qend = self.question_end_offset().unwrap_or(self.raw.len());
        resp.extend_from_slice(&self.raw[..qend]);

        // Transaction ID preserved in resp[0..2]
        // Flags: QR=1 (response), AA=1, RD=1, RA=1, RCODE=3 (NXDOMAIN) -> 0x8183
        resp[2] = 0x81;
        resp[3] = 0x83;

        // ANCOUNT = 0
        resp[6] = 0x00;
        resp[7] = 0x00;
        // NSCOUNT = 0
        resp[8] = 0x00;
        resp[9] = 0x00;
        // ARCOUNT = 0
        resp[10] = 0x00;
        resp[11] = 0x00;

        resp
    }

    /// Synthesize a 0.0.0.0 (Sinkhole / Blocked) A-record response
    pub fn build_blocked_a_response(&self) -> Vec<u8> {
        let qend = self.question_end_offset().unwrap_or(self.raw.len());
        let mut resp = Vec::with_capacity(qend + 16);
        resp.extend_from_slice(&self.raw[..qend]);

        // Flags: QR=1, AA=1, RD=1, RA=1, RCODE=0 (NOERROR) -> 0x8180
        resp[2] = 0x81;
        resp[3] = 0x80;

        // ANCOUNT = 1
        resp[6] = 0x00;
        resp[7] = 0x01;
        // NSCOUNT = 0, ARCOUNT = 0
        resp[8] = 0x00;
        resp[9] = 0x00;
        resp[10] = 0x00;
        resp[11] = 0x00;

        // Append Answer RR: Pointer to question name (0xC00C), Type A (1), Class IN (1), TTL 300 (0x0000012C), Length 4, IP 0.0.0.0
        resp.extend_from_slice(&[
            0xC0, 0x0C,             // Pointer to root question name
            0x00, 0x01,             // Type A
            0x00, 0x01,             // Class IN
            0x00, 0x00, 0x01, 0x2C, // TTL = 300 seconds
            0x00, 0x04,             // RDLENGTH = 4
            0x00, 0x00, 0x00, 0x00  // 0.0.0.0
        ]);

        resp
    }

    fn question_end_offset(&self) -> Option<usize> {
        if self.raw.len() < 12 {
            return None;
        }
        let qdcount = u16::from_be_bytes([self.raw[4], self.raw[5]]) as usize;
        let mut pos = 12;
        for _ in 0..qdcount {
            while pos < self.raw.len() && self.raw[pos] != 0 {
                if (self.raw[pos] & 0xC0) == 0xC0 {
                    pos += 2;
                    break;
                }
                pos += 1 + self.raw[pos] as usize;
            }
            if pos < self.raw.len() && self.raw[pos] == 0 {
                pos += 1;
            }
            pos += 4; // QTYPE & QCLASS
        }
        Some(pos.min(self.raw.len()))
    }
}
