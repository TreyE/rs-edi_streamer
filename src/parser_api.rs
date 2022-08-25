use crate::edi_delimiters::DelimiterResult;
use crate::edi_delimiters::detect_delimiters;
use crate::edi_segments::create_segment_iterator;
use crate::edi_segments::ParserIterator;
use std::io::{Read, Seek, Error};

type StreamerCreationResult<'a, T> = Result<ParserIterator<'a, T>, Error>;

pub fn create_edi_streamer<'a, T: Read + Seek>(ioish: &'a mut T) -> StreamerCreationResult<'a, T> {
  let delim_result = detect_delimiters(ioish);
  match delim_result {
    DelimiterResult::DelimiterReadError(e) => Err(e),
    DelimiterResult::DelimitersFound(d) => Ok(create_segment_iterator(ioish, d.element_delimiter, d.segment_delimiter))
  }
}