//! This is the existing test case from
//! https://raw.githubusercontent.com/veddan/rust-introsort/master/benches/bench.rs
#![feature(test)]
#![feature(unboxed_closures)]

extern crate timsort;
extern crate test;
extern crate rand;

use rand::{weak_rng, Rng};
use std::mem;
use test::Bencher;

#[inline]
fn sort<T: Ord>(l: &mut [T]) {
    l.sort();
}

type BigSortable = (u64,u64,u64,u64);

macro_rules! bench_random(
    ($name: ident, $sortfun: ident, $typ: ty, $n: expr) => (
        #[bench]
        fn $name(b: &mut Bencher) {
            let mut rng = weak_rng();
            b.iter(|| {
                let mut v = rng.gen_iter::<$typ>().take($n).collect::<Vec<$typ>>();
                $sortfun(&mut v[..]);
            });
            b.bytes = $n * mem::size_of::<$typ>() as u64;
        }
    )
);

bench_random!(sort_tiny_random_small, sort, u8, 5);
bench_random!(sort_tiny_random_medium, sort, u8, 100);
bench_random!(sort_tiny_random_large, sort, u8, 10_000);

bench_random!(sort_random_small, sort, u64, 5);
bench_random!(sort_random_medium, sort, u64, 100);
bench_random!(sort_random_large, sort, u64, 10_000);

bench_random!(sort_big_random_small, sort, BigSortable, 5);
bench_random!(sort_big_random_medium, sort, BigSortable, 100);
bench_random!(sort_big_random_large, sort, BigSortable, 10_000);

#[bench]
fn sort_sorted(b: &mut Bencher) {
    let mut v: Vec<_> = (0 .. 10000isize).collect();
    b.iter(|| {
        sort(&mut v[..]);
    });
    b.bytes = (v.len() * mem::size_of_val(&v[0])) as u64;
}

#[bench]
fn sort_big_sorted(b: &mut Bencher) {
    let mut v: Vec<_> = (0 .. 10000usize).map(|i| (i, i, i, i)).collect();
    b.iter(|| {
        sort(&mut v[..]);
    });
    b.bytes = (v.len() * mem::size_of_val(&v[0])) as u64;
}

#[bench]
fn sort_few_unique(b: &mut Bencher) {
    let mut v = Vec::new();
    for i in (0u32 .. 10) {
        for _ in (0usize .. 100) {
            v.push(i);
        }
    }
    let mut rng = weak_rng();
    b.iter(||{
        rng.shuffle(&mut v[..]);
        sort(&mut v[..]);
    });
    b.bytes = (v.len() * mem::size_of_val(&v[0])) as u64;
}

#[bench]
fn sort_equals(b: &mut Bencher) {
    let mut v = vec![1u64; 1000];
    b.iter(|| {
        sort(&mut v[..]);
    });
    b.bytes = (v.len() * mem::size_of_val(&v[0])) as u64;
}

#[bench]
fn sort_huge(b: &mut Bencher) {
    let mut rng = weak_rng();
    let n = 100_000;
    let mut v = rng.gen_iter::<i64>().take(n).collect::<Vec<i64>>();
    b.iter(|| {
        rng.shuffle(&mut v[..]);
        sort(&mut v[..]);
    });
    b.bytes = (n * mem::size_of::<i64>()) as u64;
}

#[bench]
fn sort_partially_sorted(b: &mut Bencher) {
    fn partially_sort<T: Ord+::std::fmt::Display>(v: &mut [T]) {
        let s = v.len() / 100;
        if s == 0 { return; }
        let mut sorted = true;
        for c in v.chunks_mut(s) {
            if sorted { sort(&mut c[..]); }
            sorted = !sorted;
        }
    }
    let mut rng = weak_rng();
    let n = 10_000;
    let mut v = rng.gen_iter::<i64>().take(n).collect::<Vec<i64>>();
    rng.shuffle(&mut v[..]);
    partially_sort(&mut v[..]);
    b.iter(|| {
        let mut v2 = v.clone();
        sort(&mut v2[..]);
    });
    b.bytes = (n * mem::size_of::<i64>()) as u64;
}

#[bench]
fn sort_strings(b: &mut Bencher) {
    let mut rng = weak_rng();
    let n = 10_000usize;
    let mut v = Vec::with_capacity(n);
    let mut bytes = 0;
    for _ in (0 .. n) {
        let len = rng.gen_range(0, 60);
        bytes += len;
        let mut s = String::with_capacity(len);
        if len == 0 {
            v.push(s);
            continue;
        }
        for _ in (0 .. len) {
            s.push(rng.gen_range(b'a', b'z') as char);
        }
        v.push(s);
    }

    b.iter(|| {
        rng.shuffle(&mut v[..]);
        sort(&mut v[..]);
    });
    b.bytes = bytes as u64;
}
