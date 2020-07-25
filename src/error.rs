#[derive(Debug)]
pub enum IpseError {
    NoneOrder,
    DataInvalid,
    IO(std::io::Error),
    Sqlite(rusqlite::Error),
    IpfsResp(ipfs_api::response::Error),
    Substrate(substrate_subxt::Error),
}

impl From<std::io::Error> for IpseError {
    fn from(err: std::io::Error) -> Self {
        IpseError::IO(err)
    }
}

impl From<rusqlite::Error> for IpseError {
    fn from(err: rusqlite::Error) -> Self {
        IpseError::Sqlite(err)
    }
}

impl From<ipfs_api::response::Error> for IpseError {
    fn from(err: ipfs_api::response::Error) -> Self {
        IpseError::IpfsResp(err)
    }
}

impl From<substrate_subxt::Error> for IpseError {
    fn from(err: substrate_subxt::Error) -> Self {
        IpseError::Substrate(err)
    }
}
