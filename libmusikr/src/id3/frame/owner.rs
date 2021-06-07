use crate::id3::frame::string::{self, Encoding};
use crate::id3::frame::{FrameHeader, Id3Frame};
use std::fmt::{self, Display, Formatter};

pub struct OwnershipFrame {
    header: FrameHeader,
    encoding: Encoding,
    price_paid: String,
    purchase_date: String,
    seller: String
}

impl OwnershipFrame {
    pub(crate) fn new(header: FrameHeader, data: &[u8]) -> Option<Self> {
        let encoding = Encoding::new(*data.get(0)?);

        if data.len() < encoding.nul_size() + 9 {
            return None;
        }

        let (price_paid, paid_size) = string::get_terminated_string(Encoding::Utf8, &data[1..]);
        let purchase_date = string::get_string(Encoding::Utf8, &data[paid_size..paid_size + 9]);
        let seller = string::get_string(encoding, &data[paid_size + 9..]);

        Some(OwnershipFrame {
            header,
            encoding,
            price_paid,
            purchase_date,
            seller
        })
    }
}

impl Id3Frame for OwnershipFrame {
    fn id(&self) -> &String {
        &self.header.frame_id
    }

    fn size(&self) -> usize {
        self.header.frame_size
    }
}

impl Display for OwnershipFrame {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        if !self.seller.is_empty() {
            write![f, "{} [", self.seller]?;

            if !self.price_paid.is_empty() {
                write![f, "{}, ", self.price_paid]?;
            }

            write![f, "{}]", self.purchase_date]?;
        } else {
            if !self.price_paid.is_empty() {
                write![f, "{}, ", self.price_paid]?;
            }
        
            write![f, "{}", self.purchase_date]?;
        }

        Ok(())
    }
}

