use zebra_chain::IndexedBlockHeader;
use {BlockHeaderProvider, BlockRef};

pub struct BlockIterator<'a> {
    block: u32,
    period: u32,
    headers: &'a BlockHeaderProvider,
}

impl<'a> BlockIterator<'a> {
    pub fn new(block: u32, period: u32, headers: &'a BlockHeaderProvider) -> Self {
        BlockIterator {
            block: block,
            period: period,
            headers: headers,
        }
    }
}

impl<'a> Iterator for BlockIterator<'a> {
    type Item = (u32, IndexedBlockHeader);

    fn next(&mut self) -> Option<Self::Item> {
        let result = self.headers.block_header(BlockRef::Number(self.block));
        let block = self.block;
        self.block += self.period;
        result.map(|header| (block, header))
    }
}
