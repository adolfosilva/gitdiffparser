use std::env;
use std::fs::File;
use std::io::{self, BufRead};

use gitdiffparser::aggregator;
use gitdiffparser::line_parser;

fn main() {
    let args: Vec<String> = env::args().collect();

    let file = File::open(&args[1]).unwrap();
    let lines = io::BufReader::new(file).lines().map(|l| l.unwrap());

    /*
    for line in lines {
        println!("{:?}", line);
    }
    */

    let lines = line_parser::parse_lines(lines).unwrap();
    //println!("{:?}", lines.len());
    //println!("{:?}", lines[0]);
    let x = aggregator::aggregator(&lines);
    println!("{:?}", x.len());
}
