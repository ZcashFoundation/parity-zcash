use canon::CanonHeader;
use deployments::Deployments;
use error::Error;
use network::ConsensusParams;
use storage::BlockHeaderProvider;
use timestamp::median_timestamp;
use work::work_required;

pub struct HeaderAcceptor<'a> {
    pub version: HeaderVersion<'a>,
    pub work: HeaderWork<'a>,
    pub median_timestamp: HeaderMedianTimestamp<'a>,
}

impl<'a> HeaderAcceptor<'a> {
    pub fn new<D: AsRef<Deployments>>(
        store: &'a BlockHeaderProvider,
        consensus: &'a ConsensusParams,
        header: CanonHeader<'a>,
        height: u32,
        time: u32,
        deployments: D,
    ) -> Self {
        let csv_active = deployments.as_ref().csv(height, store, consensus);
        HeaderAcceptor {
            work: HeaderWork::new(header, store, height, time, consensus),
            median_timestamp: HeaderMedianTimestamp::new(header, store, csv_active),
            version: HeaderVersion::new(header, height, consensus),
        }
    }

    pub fn check(&self) -> Result<(), Error> {
        self.version.check()?;
        self.work.check()?;
        self.median_timestamp.check()?;
        Ok(())
    }
}

/// Conforms to BIP90
/// https://github.com/bitcoin/bips/blob/master/bip-0090.mediawiki
pub struct HeaderVersion<'a> {
    header: CanonHeader<'a>,
    _height: u32,
    _consensus_params: &'a ConsensusParams,
}

impl<'a> HeaderVersion<'a> {
    fn new(header: CanonHeader<'a>, height: u32, consensus_params: &'a ConsensusParams) -> Self {
        HeaderVersion {
            header: header,
            _height: height,
            _consensus_params: consensus_params,
        }
    }

    fn check(&self) -> Result<(), Error> {
        if self.header.raw.version < 4 {
            Err(Error::OldVersionBlock)
        } else {
            Ok(())
        }
    }
}

pub struct HeaderWork<'a> {
    header: CanonHeader<'a>,
    store: &'a BlockHeaderProvider,
    height: u32,
    time: u32,
    consensus: &'a ConsensusParams,
}

impl<'a> HeaderWork<'a> {
    fn new(
        header: CanonHeader<'a>,
        store: &'a BlockHeaderProvider,
        height: u32,
        time: u32,
        consensus: &'a ConsensusParams,
    ) -> Self {
        HeaderWork {
            header: header,
            store: store,
            height: height,
            time: time,
            consensus: consensus,
        }
    }

    fn check(&self) -> Result<(), Error> {
        let previous_header_hash = self.header.raw.previous_header_hash.clone();
        let work = work_required(
            previous_header_hash,
            self.time,
            self.height,
            self.store,
            self.consensus,
        );
        if work == self.header.raw.bits {
            Ok(())
        } else {
            Err(Error::Difficulty {
                expected: work,
                actual: self.header.raw.bits,
            })
        }
    }
}

pub struct HeaderMedianTimestamp<'a> {
    header: CanonHeader<'a>,
    store: &'a BlockHeaderProvider,
    active: bool,
}

impl<'a> HeaderMedianTimestamp<'a> {
    fn new(header: CanonHeader<'a>, store: &'a BlockHeaderProvider, csv_active: bool) -> Self {
        HeaderMedianTimestamp {
            header: header,
            store: store,
            active: csv_active,
        }
    }

    fn check(&self) -> Result<(), Error> {
        if self.active && self.header.raw.time <= median_timestamp(&self.header.raw, self.store) {
            Err(Error::Timestamp)
        } else {
            Ok(())
        }
    }
}
