use std::{collections::HashMap, path::PathBuf};

pub type TermFreq = HashMap<String, usize>;
pub type TermFreqIndex  = HashMap<PathBuf, TermFreq>;

pub struct Lexer<'a> {
    content: &'a [char],
}

impl <'a> Lexer<'a> {
    pub fn new(content: &'a [char]) -> Self {
        Self { content }
    }

    fn trim_left(&mut self) {
        while self.content.len() > 0 && self.content[0].is_whitespace(){
            self.content = &self.content[1..];
        }
    }

    fn chop(&mut self, n: usize) -> &'a [char]{
        let result = &self.content[0..n];
        self.content = &self.content[n..];
        result
    }

    fn chop_while<T>(&mut self, mut predicate: T) -> &'a[char] where T : FnMut(&char) -> bool {
        let mut n = 0;
        while n<self.content.len() && predicate(&self.content[n]){
            n+=1
        }
        return self.chop(n);
    }

    fn next_token(&mut self) -> Option<String> {
        self.trim_left();
        if self.content.len() == 0 {
            return None;
        }

        if self.content[0].is_numeric(){
            

            return Some(self.chop_while(|x| x.is_numeric()).into_iter().collect::<String>());

        }
        if self.content[0].is_alphabetic(){
        
            return Some(self.chop_while(|x| x.is_alphanumeric()).into_iter().map(|x| x.to_ascii_uppercase()).collect::<String>());
        }
        Some(self.chop(1).into_iter().collect::<String>())
    }
    
}

impl<'a> Iterator for Lexer<'a> {
    type Item = String;

    fn next(&mut self) -> Option<Self::Item> {
        self.next_token()
    }
}

fn _index_document(_doc_content: &str) -> HashMap<String, usize> {
    todo!()
} 

