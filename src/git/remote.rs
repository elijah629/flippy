use gix::protocol::handshake;

pub fn by_name_or_url<'repo>(
    repo: &'repo gix::Repository,
    name_or_url: Option<&str>,
) -> anyhow::Result<gix::Remote<'repo>> {
    repo.find_fetch_remote(name_or_url.map(Into::into))
        .map_err(Into::into)
}

pub fn ref_to_string(r: &handshake::Ref) -> String {
    match r {
        handshake::Ref::Direct {
            full_ref_name: path,
            object,
        } => format!("{object} {path}"),
        handshake::Ref::Peeled {
            full_ref_name: path,
            tag,
            object,
        } => format!("{tag} {path} object:{object}"),
        handshake::Ref::Symbolic {
            full_ref_name: path,
            tag,
            target,
            object,
        } => match tag {
            Some(tag) => {
                format!("{tag} {path} symref-target:{target} peeled:{object}")
            }
            None => {
                format!("{object} {path} symref-target:{target}")
            }
        },
        handshake::Ref::Unborn {
            full_ref_name,
            target,
        } => {
            format!("unborn {full_ref_name} symref-target:{target}")
        }
    }
}
