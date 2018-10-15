#![feature(test, duration_float)]
extern crate test;
extern crate abomonation;
extern crate memmap;
use std::fs::File;
use std::io::{BufReader, BufRead, BufWriter};
use std::env;
use std::str;
use std::time::Instant;

pub fn encode_abom_old(in_filename: &str, out_filename: &str) {
    let mut lines: Vec<String> = Vec::new();
    let in_file = File::open(in_filename).unwrap();
    for line in BufReader::new(in_file).lines() {
        lines.push(line.unwrap());
    }
    let out_file = File::create(out_filename).unwrap();
    unsafe { abomonation::encode(&lines, &mut BufWriter::new(out_file)).unwrap(); }
}

pub fn encode_abom(in_filename: &str, out_filename: &str) {
    let in_file = File::open(in_filename).unwrap();
    let mmap = unsafe { memmap::Mmap::map(&in_file).unwrap() };

    let mut start_of_line_idx = 0;
    let mut end_of_line_idx = 0;
    let mut lines: Vec<String> = Vec::new();
    while start_of_line_idx < mmap.len() {
        while mmap[end_of_line_idx] != 10 { // new line
            end_of_line_idx += 1;
        }
        let s = unsafe { str::from_utf8_unchecked(&mmap[start_of_line_idx..end_of_line_idx]) };
        //let s = str::from_utf8(&mmap[start_of_line_idx..end_of_line_idx]).unwrap();
        lines.push(s.to_owned());
        start_of_line_idx = end_of_line_idx + 1;
        end_of_line_idx = start_of_line_idx;
    }
    let out_file = File::create(out_filename).unwrap();
    unsafe { abomonation::encode(&lines, &mut BufWriter::new(out_file)).unwrap(); }
}

fn decode_abom<R>(in_filename: &str, then: impl FnOnce(&Vec<String>) -> R) -> R {
    let in_file = File::open(in_filename).unwrap();
    let mut mmap = unsafe { memmap::MmapOptions::new().map_copy(&in_file).unwrap() };
    let (result, remaining) = unsafe { abomonation::decode::<Vec<String>>(&mut mmap[..]).unwrap() };
    assert_eq!(remaining.len(), 0);
    then(result)
}

fn byte_sum(s: &str) -> u32 {
    let mut res: u32 = 0;
    for b in s.bytes() {
        res = res.wrapping_add(b as u32);
    }
    res
}

pub fn decode_abom_and_get_nth_byte_sum(in_filename: &str, n: usize) -> u32 {
    decode_abom(in_filename, |words| {
        byte_sum(&words[n])
    })
}

pub fn decode_abom_and_get_all_byte_sum(in_filename: &str) -> u32 {
    decode_abom(in_filename, |words| {
        let mut res: u32 = 0;
        for word in words {
            //test::black_box(word);
            res = res.wrapping_add(byte_sum(word));
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
        let mut out: Vec<u8> = Vec::new();
        //let out_file = File::create("/dev/null").unwrap();
        //let mut out = BufWriter::new(out_file);
        unsafe { abomonation::encode(&lines, &mut out).unwrap(); }
        test::black_box(&out);
    }
    println!("{}", start.elapsed().as_float_secs());
}

fn main() {
    let mode = env::args().nth(1).unwrap();
    let in_filename = env::args().nth(2).unwrap();
    match &mode[..] {
        "encode-old" => encode_abom_old(&in_filename, &env::args().nth(3).unwrap()),
        "encode" => encode_abom(&in_filename, &env::args().nth(3).unwrap()),
        "decode-nth" => println!("{}", decode_abom_and_get_nth_byte_sum(&in_filename, env::args().nth(3).unwrap().parse::<usize>().unwrap())),
        "decode-all" => println!("{}", decode_abom_and_get_all_byte_sum(&in_filename)),
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
        b.iter(|| encode_abom("/tmp/manywords", "/dev/null"));
    }

    #[bench]
    fn bench_decode_10000th(b: &mut Bencher) {
        b.iter(|| decode_abom_and_get_nth_byte_sum("/tmp/manywords-encoded-abom", 10000))
    }

    #[bench]
    fn bench_decode_all(b: &mut Bencher) {
        b.iter(|| decode_abom_and_get_all_byte_sum("/tmp/manywords-encoded-abom"))
    }
}

