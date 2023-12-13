use std::fmt;
use std::fmt::Formatter;

pub struct ValueError {
    pub(crate) msg: String,
}

impl fmt::Display for ValueError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.write_str(self.msg.as_str())
    }
}
