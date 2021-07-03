use bytes::Bytes;
use chain::{
    sync_packet::{PacketInfo, SyncPacket},
    ChainSync, PeerInfo,
};
use network::PeerId;
use rlp::{DecoderError, Rlp, RlpStream};

pub type RequestId = u64;

// Separate the eth/66 request id from a packet, if it exists.
pub fn strip_request_id<'a>(
    data: &'a [u8],
    sync: &ChainSync,
    peer: &PeerId,
    packet_id: &SyncPacket,
) -> Result<(Rlp<'a>, Option<RequestId>), DecoderError> {
    let protocol_version = if let Some(peer_info) = sync.peers.get(peer) {
        peer_info.protocol_version
    } else {
        trace!(
            "Peer info missing for peer {}, assuming protocol version 66",
            peer
        );
        66
    };

    let has_request_id = protocol_version >= 66 && packet_id.has_request_id_in_eth_66();

    do_strip_request_id(data, has_request_id)
}

fn do_strip_request_id<'a>(
    data: &'a [u8],
    has_request_id: bool,
) -> Result<(Rlp<'a>, Option<RequestId>), DecoderError> {
    let rlp = Rlp::new(data);

    if has_request_id {
        let request_id: RequestId = rlp.val_at(0)?;
        let stripped_rlp = rlp.at(1)?;
        Ok((stripped_rlp, Some(request_id)))
    } else {
        Ok((rlp, None))
    }
}

// Add a given eth/66 request id to a packet being built.
pub fn prepend_request_id(rlp: RlpStream, request_id: Option<RequestId>) -> RlpStream {
    match request_id {
        Some(ref id) => {
            let mut stream = RlpStream::new_list(2);
            stream.append(id);
            stream.append_raw(&rlp.out(), 1);
            stream
        }
        None => rlp,
    }
}

/// Prepend a new eth/66 request id to the packet if appropriate.
pub fn generate_request_id(
    packet: Bytes,
    peer: &PeerInfo,
    packet_id: SyncPacket,
) -> (Bytes, Option<RequestId>) {
    if peer.protocol_version >= 66 && packet_id.has_request_id_in_eth_66() {
        do_generate_request_id(&packet)
    } else {
        (packet, None)
    }
}

fn do_generate_request_id(packet: &Bytes) -> (Bytes, Option<RequestId>) {
    let request_id: RequestId = rand::random();

    let mut rlp = RlpStream::new_list(2);
    rlp.append(&request_id);
    rlp.append_raw(packet, 1);

    (rlp.out(), Some(request_id))
}

#[cfg(test)]
mod tests {
    use super::*;
    use ethereum_types::H256;

    #[test]
    fn test_prepend_request_id() {
        let mut request = RlpStream::new_list(2);
        request.append(&H256::from_low_u64_be(1));
        request.append(&H256::from_low_u64_be(2));

        let with_id = prepend_request_id(request, Some(10));
        let rlp = Rlp::new(with_id.as_raw());
        let recovered_id: RequestId = rlp.val_at(0).unwrap();
        let recovered_request: Vec<H256> = rlp.at(1).unwrap().as_list().unwrap();

        assert_eq!(recovered_id, 10);
        assert_eq!(
            recovered_request,
            [H256::from_low_u64_be(1), H256::from_low_u64_be(2)]
        );
    }

    #[test]
    fn test_strip_request_id() {
        let request = vec![
            H256::from_low_u64_be(1),
            H256::from_low_u64_be(2),
            H256::from_low_u64_be(3),
        ];

        let mut request_with_id = RlpStream::new_list(2);
        request_with_id.append(&20u64);
        request_with_id.append_list(&request);
        let data = request_with_id.out();

        let (rlp, id) = do_strip_request_id(&data, true).unwrap();

        assert_eq!(id, Some(20));
        assert_eq!(rlp.as_list::<H256>().unwrap(), request);
    }

    #[test]
    fn test_generate_request_id() {
        let request = vec![
            H256::from_low_u64_be(1),
            H256::from_low_u64_be(2),
            H256::from_low_u64_be(3),
        ];

        let mut stream = RlpStream::new_list(3);
        for hash in &request {
            stream.append(hash);
        }
        let data = stream.out();

        let (new_data, id) = do_generate_request_id(&data);

        let recovered = Rlp::new(&new_data);
        let recovered_id: RequestId = recovered.val_at(0).unwrap();
        let recovered_request: Vec<H256> = recovered.at(1).unwrap().as_list().unwrap();
        assert_eq!(recovered_id, id.unwrap());
        assert_eq!(recovered_request, request);
    }
}
