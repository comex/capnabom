#![feature(test, duration_float)]
extern crate test;
extern crate capnp;
pub mod foo_capnp {
    include!(concat!(env!("OUT_DIR"), "/foo_capnp.rs"));
}

extern crate memmap;
use std::fs::File;
use std::io::{BufReader, BufRead, BufWriter};
use std::env;
use std::time::Instant;
use std::io::{Seek, SeekFrom};

pub fn encode_capn_old(in_filename: &str, out_filename: &str, use_first_segment_words: bool) {
    let mut lines: Vec<String> = Vec::new();
    let mut in_file = File::open(in_filename).unwrap();
    let in_file_len = in_file.seek(SeekFrom::End(0)).unwrap();
    in_file.seek(SeekFrom::Start(0)).unwrap();
    for line in BufReader::new(in_file).lines() {
        lines.push(line.unwrap());
    }
    let first_segment_words = 2 + 2 * (lines.len() as u64) + in_file_len / 8;
    let mut builder = if use_first_segment_words {
        capnp::message::Builder::new(
            capnp::message::HeapAllocator::new().first_segment_words(first_segment_words as u32))
    } else {
        capnp::message::Builder::new_default()
    };
    {
        let msg = builder.init_root::<foo_capnp::dictionary::Builder>();
        let mut words = msg.init_words(lines.len() as u32);
        for (i, line) in lines.iter().enumerate() {
            words.set(i as u32, line);
        }
    }
    let out_file = File::create(out_filename).unwrap();
    capnp::serialize::write_message(&mut BufWriter::new(out_file), &builder).unwrap();
}

pub fn encode_capn(in_filename: &str, out_filename: &str, use_first_segment_words: bool) {
    let in_file = File::open(in_filename).unwrap();
    let mmap = unsafe { memmap::Mmap::map(&in_file).unwrap() };

    let mut num_lines = 0;
    for c in &mmap[..] {
        if *c == 10 { // new line
            num_lines += 1;
        }
    }

    // conservative overestimate for how much we need to allocate
    let first_segment_words = 2 + 2 * num_lines + mmap.len() / 8;

    let mut builder = if use_first_segment_words {
        capnp::message::Builder::new(
            capnp::message::HeapAllocator::new().first_segment_words(first_segment_words as u32))
    } else {
        capnp::message::Builder::new_default()
    };
    {
        let msg = builder.init_root::<foo_capnp::dictionary::Builder>();
        let mut words = msg.init_words(num_lines as u32);
        let mut start_of_line_idx = 0;
        let mut end_of_line_idx = 0;
        let mut idx = 0;
        while start_of_line_idx < mmap.len() {
            while mmap[end_of_line_idx] != 10 { // new line
                end_of_line_idx += 1;
            }
            words.set(idx as u32,
                      unsafe { std::str::from_utf8_unchecked(&mmap[start_of_line_idx..end_of_line_idx])});
            idx += 1;
            start_of_line_idx = end_of_line_idx + 1;
            end_of_line_idx = start_of_line_idx;
        }
    }
    let out_file = File::create(out_filename).unwrap();
    capnp::serialize::write_message(&mut BufWriter::new(out_file), &builder).unwrap();
}

fn decode_capn<R>(in_filename: &str, then: impl FnOnce(capnp::text_list::Reader) -> R) -> R {
    let in_file = File::open(in_filename).unwrap();
    let mmap = unsafe { memmap::Mmap::map(&in_file).unwrap() };
    let reader = capnp::serialize::read_message_from_words(
        unsafe { capnp::Word::bytes_to_words(&mmap[..]) },
        *capnp::message::ReaderOptions::new().traversal_limit_in_words(1000000000)
    ).unwrap();
    let msg = reader.get_root::<foo_capnp::dictionary::Reader>().unwrap();
    let words = msg.get_words().unwrap();
    then(words)
}

fn byte_sum(s: &str) -> u32 {
    let mut res: u32 = 0;
    for b in s.bytes() {
        res = res.wrapping_add(b as u32);
    }
    res
}

pub fn decode_capn_and_get_nth_byte_sum(in_filename: &str, n: usize) -> u32 {
    decode_capn(in_filename, |words| {
        let word = words.get(n as u32).unwrap();
        byte_sum(word)
    })
}

pub fn decode_capn_and_get_all_byte_sum(in_filename: &str) -> u32 {
    decode_capn(in_filename, |words| {
        let mut res: u32 = 0;
        for word in words {
            let word = word.unwrap();
            res = res.wrapping_add(byte_sum(word));
            //test::black_box(word);
        }
        res
    })
}

pub fn test_encode_pure(in_filename: &str) {
    let mut lines: Vec<String> = Vec::new();
    let in_file = File::open(in_filename).unwrap();
    for line in BufReader::new(in_file).lines() {
        lines.push(line.unwrap());
    }
    // since cargo bench wants to run 300 times which is way too slow
    let start = Instant::now();
    for _ in 0..10 {
        let mut builder = capnp::message::Builder::new_default();
        {
            let msg = builder.init_root::<foo_capnp::dictionary::Builder>();
            let mut words = msg.init_words(lines.len() as u32);
            for (i, line) in lines.iter().enumerate() {
                words.set(i as u32, line);
            }
        }
        //let mut out: Vec<u8> = Vec::new();
        let out_file = File::create("/dev/null").unwrap();
        let mut out = BufWriter::new(out_file);
        capnp::serialize::write_message(&mut out, &builder).unwrap();
        test::black_box(&out);
    }
    println!("{}", start.elapsed().as_float_secs());
}

fn main() {
    let mode = env::args().nth(1).unwrap();
    let in_filename = env::args().nth(2).unwrap();
    match &mode[..] {
        "encode-old" => encode_capn_old(&in_filename, &env::args().nth(3).unwrap(), /*use_first_segment_words*/ false),
        "encode-old-fsw" => encode_capn_old(&in_filename, &env::args().nth(3).unwrap(), /*use_first_segment_words*/ true),
        "encode-no-fsw" => encode_capn(&in_filename, &env::args().nth(3).unwrap(), /*use_first_segment_words*/ false),
        "encode" => encode_capn(&in_filename, &env::args().nth(3).unwrap(), /*use_first_segment_words*/ true),
        "decode-nth" => println!("{}", decode_capn_and_get_nth_byte_sum(&in_filename, env::args().nth(3).unwrap().parse::<usize>().unwrap())),
        "decode-all" => println!("{}", decode_capn_and_get_all_byte_sum(&in_filename)),
        "encode-pure" => test_encode_pure(&in_filename),
        _ => panic!("?")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    extern crate test;
    use self::test::Bencher;

    #[bench]
    fn bench_encode(b: &mut Bencher) {
        b.iter(|| encode_capn("/tmp/manywords", "/dev/null"));
    }

    #[bench]
    fn bench_decode_10000th(b: &mut Bencher) {
        b.iter(|| decode_capn_and_get_nth_byte_sum("/tmp/manywords-encoded-capn", 10000))
    }

    #[bench]
    fn bench_decode_all(b: &mut Bencher) {
        b.iter(|| decode_capn_and_get_all_byte_sum("/tmp/manywords-encoded-capn"))
    }
}



