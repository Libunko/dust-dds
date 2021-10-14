use crate::structure::types::Guid;

use super::{reader::RtpsReader, writer_proxy::RtpsWriterProxy};

pub struct RtpsStatefulReader<L, C, P> {
    pub rtps_reader: RtpsReader<L, C>,
    pub matched_writers: P,
}

pub trait RtpsStatefulReaderOperations<L> {
    fn matched_writer_add(&mut self, a_writer_proxy: RtpsWriterProxy<L>);
    fn matched_writer_remove(&mut self, writer_proxy_guid: &Guid);
    fn matched_writer_lookup(&self, a_writer_guid: &Guid) -> Option<&RtpsWriterProxy<L>>;
}
