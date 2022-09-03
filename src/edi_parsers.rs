use crate::edi_segments::Segment;
use crate::edi_segments::ParserIterator;
use crate::edi_delimiters::DelimiterResult;
use crate::edi_delimiters::detect_delimiters;
use crate::edi_segments::create_segment_iterator;
use crate::edi_constants::{ST_TAG, SE_TAG, GS_TAG, GE_TAG, IEA_TAG, ISA_TAG};
use std::io::Error;
use std::io::Read;
use std::io::Seek;

type StreamerCreationResult<'a, T> = Result<ParserIterator<'a, T>, Error>;

#[allow(clippy::needless_lifetimes)]
pub fn create_edi_streamer<'a, T: Read + Seek>(ioish: &'a mut T) -> StreamerCreationResult<'a, T> {
  let delim_result = detect_delimiters(ioish);
  match delim_result {
    DelimiterResult::DelimiterReadError(e) => Err(e),
    DelimiterResult::DelimitersFound(d) => Ok(create_segment_iterator(ioish, d.element_delimiter, d.segment_delimiter))
  }
}

pub trait StreamParser {
  fn segment(&self, segment: &Segment);

  fn interchange_start(&self, segment: &Segment);
  fn interchange_end(&self, segment: Option<&Segment>);

  fn functional_group_start(&self, segment: &Segment);
  fn functional_group_end(&self, segment: Option<&Segment>);

  fn transaction_start(&self, segment: &Segment);
  fn transaction_end(&self, segment: Option<&Segment>);

  fn stream_end(&self);
  fn error(&self, error: Error);

  fn in_interchange(&self) -> bool;
  fn in_functional_group(&self) -> bool;
  fn in_transaction(&self) -> bool;
}

pub fn execute_streaming_parser<T: Read, U: StreamParser>(parser_iterator: &mut ParserIterator<T>, stream_parser: &mut U) {
  let mut pr : Option<Result<Segment, Error>> = parser_iterator.next();
  loop {
    match pr {
      None => break,
      Some(Err(e)) => {
        stream_parser.error(e);
        return ;
      }
      Some(Ok(segment)) => consume_segment(stream_parser, &segment)
    }
    pr = parser_iterator.next();
  }
  complete_parsing(stream_parser);
}

fn complete_parsing<T: StreamParser>(stream_parser: &mut T) {
  if stream_parser.in_transaction() {
    stream_parser.transaction_end(None);
    stream_parser.functional_group_end(None);
    stream_parser.interchange_end(None);
  } else if stream_parser.in_functional_group() {
    stream_parser.functional_group_end(None);
    stream_parser.interchange_end(None);
  } else if stream_parser.in_interchange() {
    stream_parser.interchange_end(None);
  }
  stream_parser.stream_end();
}

fn consume_segment<T: StreamParser>(stream_parser: &mut T, segment: &Segment) {
  if stream_parser.in_transaction() {
    consume_segment_in_transaction(stream_parser, segment);
  } else if stream_parser.in_functional_group() {
    consume_segment_in_functional_group(stream_parser, segment);
  } else if stream_parser.in_interchange() {
    consume_segment_in_interchange(stream_parser, segment);
  } else {
    consume_segment_in_nothing(stream_parser, segment);
  }
}

fn consume_segment_in_nothing<T: StreamParser>(stream_parser: &mut T, segment: &Segment) {
  let tag_compare = segment.tag.as_slice();
  if ISA_TAG.eq(tag_compare) {
    stream_parser.interchange_start(segment);
  }
  stream_parser.segment(segment);
}

fn consume_segment_in_transaction<T: StreamParser>(stream_parser: &mut T, segment: &Segment) {
  let tag_compare = segment.tag.as_slice();
  if SE_TAG.eq(tag_compare) {
    stream_parser.segment(segment);
    stream_parser.transaction_end(Some(segment));
  } else if ST_TAG.eq(tag_compare) {
    stream_parser.transaction_end(None);
    stream_parser.transaction_start(segment);
    stream_parser.segment(segment);
  } else if GE_TAG.eq(tag_compare) {
    stream_parser.transaction_end(None);
    stream_parser.segment(segment);
    stream_parser.functional_group_end(Some(segment));
  } else if GS_TAG.eq(tag_compare) {
    stream_parser.transaction_end(None);
    stream_parser.functional_group_end(None);
    stream_parser.functional_group_start(segment);
    stream_parser.segment(segment);
  } else if IEA_TAG.eq(tag_compare) {
    stream_parser.transaction_end(None);
    stream_parser.functional_group_end(None);
    stream_parser.segment(segment);
    stream_parser.interchange_end(Some(segment));
  } else if ISA_TAG.eq(tag_compare) {
    stream_parser.transaction_end(None);
    stream_parser.functional_group_end(None);
    stream_parser.interchange_end(None);
    stream_parser.interchange_start(segment);
    stream_parser.segment(segment);
  } else {
    stream_parser.segment(segment);
  }
}

fn consume_segment_in_functional_group<T: StreamParser>(stream_parser: &mut T, segment: &Segment) {
  let tag_compare = segment.tag.as_slice();
  if GS_TAG.eq(tag_compare) {
    stream_parser.functional_group_end(None);
    stream_parser.functional_group_start(segment);
    stream_parser.segment(segment);
  } else if GE_TAG.eq(tag_compare) {
    stream_parser.segment(segment);
    stream_parser.functional_group_end(Some(segment));
  } else if ST_TAG.eq(tag_compare) {
    stream_parser.transaction_start(segment);
    stream_parser.segment(segment);
  } else if IEA_TAG.eq(tag_compare) {
    stream_parser.functional_group_end(None);
    stream_parser.segment(segment);
    stream_parser.interchange_end(Some(segment));
  } else {
    stream_parser.segment(segment);
  }
}

fn consume_segment_in_interchange<T: StreamParser>(stream_parser: &mut T, segment: &Segment) {
  let tag_compare = segment.tag.as_slice();
  if GS_TAG.eq(tag_compare) {
    stream_parser.functional_group_start(segment);
    stream_parser.segment(segment);
  } else if IEA_TAG.eq(segment.tag.as_slice()) {
    stream_parser.segment(segment);
    stream_parser.interchange_end(Some(segment));
  } else if ISA_TAG.eq(tag_compare) {
    stream_parser.interchange_end(None);
    stream_parser.interchange_start(segment);
    stream_parser.segment(segment);
  } else {
    stream_parser.segment(segment);
  }
}