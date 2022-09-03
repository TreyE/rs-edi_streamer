use std::io::{Read, Seek, SeekFrom, Error, ErrorKind};
use crate::edi_constants::SEGMENT_STARTERS;

pub struct Delimiters {
  pub element_delimiter: Vec<u8>,
  pub segment_delimiter: Vec<u8>
}

pub enum DelimiterResult {
  DelimiterReadError(Error),
  DelimitersFound(Delimiters)
}

pub fn detect_delimiters<T: Read + Seek>(ioish: &mut T) -> DelimiterResult {
  let pos = SeekFrom::Start(3);
  match ioish.seek(pos) {
    Ok(_) => (),
    Err(e) => return DelimiterResult::DelimiterReadError(e)
  }
  let mut buff = [0; 1];
  match ioish.read(&mut buff) {
    Ok(size) if size == 1 => (),
    Ok(_) => {
      let eof_error = Error::from(ErrorKind::UnexpectedEof);
      return DelimiterResult::DelimiterReadError(eof_error)
    },
    Err(e) => return DelimiterResult::DelimiterReadError(e)
  };
  let delim_val = buff;
  let element_delimiter = Vec::from([buff[0]]);
  let mut sd_buff = [0; 1];
  let mut read_count = 0;
  let mut delim_count = 1;
  while delim_count < 16 {
    if read_count > 212 {
      let eof_error = Error::from(ErrorKind::UnexpectedEof);
      return DelimiterResult::DelimiterReadError(eof_error)
    }
    match ioish.read(&mut sd_buff) {
      Ok(size) if size == 1 => (),
      Ok(_) => {
        let eof_error = Error::from(ErrorKind::UnexpectedEof);
        return DelimiterResult::DelimiterReadError(eof_error)
      },
      Err(e) => return DelimiterResult::DelimiterReadError(e)
    };
    if delim_val == sd_buff {
      delim_count += 1;
    }
    read_count += 1;
  }
  let mut seg_delimiter : Vec<u8> = Vec::new();
  match ioish.read(&mut sd_buff) {
    Ok(size) if size == 1 => (),
    Ok(_) => {
      let eof_error = Error::from(ErrorKind::UnexpectedEof);
      return DelimiterResult::DelimiterReadError(eof_error)
    },
    Err(e) => return DelimiterResult::DelimiterReadError(e)
  };
  match ioish.read(&mut sd_buff) {
    Ok(size) if size == 1 => (),
    Ok(_) => {
      let eof_error = Error::from(ErrorKind::UnexpectedEof);
      return DelimiterResult::DelimiterReadError(eof_error)
    },
    Err(e) => return DelimiterResult::DelimiterReadError(e)
  };
  while !SEGMENT_STARTERS.contains(&sd_buff[0]) {
    seg_delimiter.push(sd_buff[0]);
    match ioish.read(&mut sd_buff) {
      Ok(size) if size == 1 => (),
      Ok(_) => {
        return DelimiterResult::DelimitersFound(
          Delimiters {
            element_delimiter,
            segment_delimiter: seg_delimiter
          }
        )
      },
      Err(e) => return DelimiterResult::DelimiterReadError(e)
    };
  }
  if seg_delimiter.is_empty() {
    let eof_error = Error::from(ErrorKind::UnexpectedEof);
    return DelimiterResult::DelimiterReadError(eof_error)
  }
  let rewind_pos = SeekFrom::Start(0);
  match ioish.seek(rewind_pos) {
    Ok(_) => (),
    Err(e) => return DelimiterResult::DelimiterReadError(e)
  }
  DelimiterResult::DelimitersFound(
    Delimiters {
      element_delimiter,
      segment_delimiter: seg_delimiter
    }
  )
}

#[cfg(test)]
mod test {
  use super::detect_delimiters;
  use super::DelimiterResult;
  use std::io::ErrorKind;
  use std::io::Cursor;

  #[test]
  fn not_long_enough_for_field_delimiter() {
    let mut ioish = Cursor::new("".as_bytes());
    let res = detect_delimiters(&mut ioish);
    match res {
      DelimiterResult::DelimiterReadError(e) => assert_eq!(e.kind(), ErrorKind::UnexpectedEof),
      _ => panic!("Delimiters found - should have been an error instead")
    }
  }

  #[test]
  fn not_long_enough_for_segment_delimiter() {
    let mut ioish = Cursor::new("ISA*00*TSI       *01*92511930  *01*ME             *12*BRADLEY        *970815*1732*U*00201*000000050*0*T>~".as_bytes());
    let res = detect_delimiters(&mut ioish);
    match res {
      DelimiterResult::DelimiterReadError(e) => assert_eq!(e.kind(), ErrorKind::UnexpectedEof),
      _ => panic!("Delimiters found - should have been an error instead")
    }
  }

  #[test]
  fn weird_missing_delimiter() {
    let mut ioish = Cursor::new("ISA*00*TSI       *01*92511930  *01*ME             *12*BRADLEY        *970815*1732*U*00201*000000050*0*T*>ISA".as_bytes());
    let res = detect_delimiters(&mut ioish);
    match res {
      DelimiterResult::DelimiterReadError(e) => assert_eq!(e.kind(), ErrorKind::UnexpectedEof),
      _ => panic!("Delimiters found - should have been an error instead")
    }
  }

  #[test]
  fn simple_delimiter_set() {
    let mut ioish = Cursor::new("ISA*00*TSI       *01*92511930  *01*ME             *12*BRADLEY        *970815*1732*U*00201*000000050*0*T*>~".as_bytes());
    let res = detect_delimiters(&mut ioish);
    match res {
      DelimiterResult::DelimiterReadError(_) => panic!("Delimiters not found"),
      DelimiterResult::DelimitersFound(x) => {
        assert_eq!(x.element_delimiter, Vec::from([('*' as u8)]));
        assert_eq!(x.segment_delimiter, Vec::from([('~' as u8)]));
      }
    }
  }

  #[test]
  fn multibyte_delimiter_set_eof() {
    let mut ioish = Cursor::new("ISA*00*TSI       *01*92511930  *01*ME             *12*BRADLEY        *970815*1732*U*00201*000000050*0*T*>~\n".as_bytes());
    let res = detect_delimiters(&mut ioish);
    match res {
      DelimiterResult::DelimiterReadError(_) => panic!("Delimiters not found"),
      DelimiterResult::DelimitersFound(x) => {
        assert_eq!(x.element_delimiter, Vec::from([('*' as u8)]));
        assert_eq!(x.segment_delimiter, Vec::from([('~' as u8), ('\n' as u8)]));
      }
    }
  }

  #[test]
  fn multibyte_delimiter_set() {
    let mut ioish = Cursor::new("ISA*00*TSI       *01*92511930  *01*ME             *12*BRADLEY        *970815*1732*U*00201*000000050*0*T*>~\nIEA".as_bytes());
    let res = detect_delimiters(&mut ioish);
    match res {
      DelimiterResult::DelimiterReadError(_) => panic!("Delimiters not found"),
      DelimiterResult::DelimitersFound(x) => {
        assert_eq!(x.element_delimiter, Vec::from([('*' as u8)]));
        assert_eq!(x.segment_delimiter, Vec::from([('~' as u8), ('\n' as u8)]));
      }
    }
  }
}