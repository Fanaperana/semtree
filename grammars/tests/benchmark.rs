use std::collections::{HashMap, BTreeMap, VecDeque, HashSet};
use std::fmt;
use std::io::{self, Read, Write, BufRead, BufReader};
use std::fs::File;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

// ── Error Handling ──────────────────────────────────────────

#[derive(Debug)]
pub enum AppError {
    Io(io::Error),
    Parse(String),
    NotFound(String),
    InvalidInput(String),
    Internal(String),
}

impl fmt::Display for AppError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AppError::Io(err) => write!(f, "IO error: {}", err),
            AppError::Parse(msg) => write!(f, "Parse error: {}", msg),
            AppError::NotFound(key) => write!(f, "Not found: {}", key),
            AppError::InvalidInput(msg) => write!(f, "Invalid input: {}", msg),
            AppError::Internal(msg) => write!(f, "Internal error: {}", msg),
        }
    }
}

impl From<io::Error> for AppError {
    fn from(err: io::Error) -> Self {
        AppError::Io(err)
    }
}

impl std::error::Error for AppError {}

type Result<T> = std::result::Result<T, AppError>;

// ── Config ──────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct Config {
    pub database_path: PathBuf,
    pub cache_size: usize,
    pub batch_size: usize,
    pub max_retries: u32,
    pub log_level: LogLevel,
    pub tags: HashMap<String, String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LogLevel {
    Debug,
    Info,
    Warning,
    Error,
}

impl fmt::Display for LogLevel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LogLevel::Debug => write!(f, "DEBUG"),
            LogLevel::Info => write!(f, "INFO"),
            LogLevel::Warning => write!(f, "WARNING"),
            LogLevel::Error => write!(f, "ERROR"),
        }
    }
}

impl Default for Config {
    fn default() -> Self {
        Config {
            database_path: PathBuf::from("data.db"),
            cache_size: 1024,
            batch_size: 100,
            max_retries: 3,
            log_level: LogLevel::Info,
            tags: HashMap::new(),
        }
    }
}

impl Config {
    pub fn validate(&self) -> Vec<String> {
        let mut errors = Vec::new();
        if self.cache_size == 0 {
            errors.push("cache_size must be positive".to_string());
        }
        if self.batch_size == 0 {
            errors.push("batch_size must be positive".to_string());
        }
        if self.max_retries == 0 {
            errors.push("max_retries must be at least 1".to_string());
        }
        errors
    }

    pub fn with_cache_size(mut self, size: usize) -> Self {
        self.cache_size = size;
        self
    }

    pub fn with_batch_size(mut self, size: usize) -> Self {
        self.batch_size = size;
        self
    }
}

// ── LRU Cache ───────────────────────────────────────────────

pub struct LruCache<K: std::hash::Hash + Eq + Clone, V> {
    capacity: usize,
    map: HashMap<K, V>,
    order: VecDeque<K>,
}

impl<K: std::hash::Hash + Eq + Clone, V> LruCache<K, V> {
    pub fn new(capacity: usize) -> Self {
        LruCache {
            capacity,
            map: HashMap::with_capacity(capacity),
            order: VecDeque::with_capacity(capacity),
        }
    }

    pub fn get(&mut self, key: &K) -> Option<&V> {
        if self.map.contains_key(key) {
            self.order.retain(|k| k != key);
            self.order.push_back(key.clone());
            self.map.get(key)
        } else {
            None
        }
    }

    pub fn put(&mut self, key: K, value: V) {
        if self.map.contains_key(&key) {
            self.order.retain(|k| k != &key);
        } else if self.map.len() >= self.capacity {
            if let Some(evicted) = self.order.pop_front() {
                self.map.remove(&evicted);
            }
        }
        self.map.insert(key.clone(), value);
        self.order.push_back(key);
    }

    pub fn contains(&self, key: &K) -> bool {
        self.map.contains_key(key)
    }

    pub fn len(&self) -> usize {
        self.map.len()
    }

    pub fn is_empty(&self) -> bool {
        self.map.is_empty()
    }

    pub fn clear(&mut self) {
        self.map.clear();
        self.order.clear();
    }
}

// ── Record / Store ──────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct Record {
    pub id: u64,
    pub name: String,
    pub category: Category,
    pub value: f64,
    pub tags: Vec<String>,
    pub metadata: HashMap<String, String>,
    pub created_at: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Category {
    Alpha,
    Beta,
    Gamma,
    Delta,
    Epsilon,
}

impl fmt::Display for Category {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Category::Alpha => write!(f, "alpha"),
            Category::Beta => write!(f, "beta"),
            Category::Gamma => write!(f, "gamma"),
            Category::Delta => write!(f, "delta"),
            Category::Epsilon => write!(f, "epsilon"),
        }
    }
}

impl Category {
    pub fn from_str(s: &str) -> Result<Self> {
        match s {
            "alpha" => Ok(Category::Alpha),
            "beta" => Ok(Category::Beta),
            "gamma" => Ok(Category::Gamma),
            "delta" => Ok(Category::Delta),
            "epsilon" => Ok(Category::Epsilon),
            other => Err(AppError::Parse(format!("Unknown category: {}", other))),
        }
    }

    pub fn all() -> &'static [Category] {
        &[
            Category::Alpha,
            Category::Beta,
            Category::Gamma,
            Category::Delta,
            Category::Epsilon,
        ]
    }
}

impl Record {
    pub fn new(id: u64, name: String, category: Category, value: f64) -> Self {
        Record {
            id,
            name,
            category,
            value,
            tags: Vec::new(),
            metadata: HashMap::new(),
            created_at: 0,
        }
    }

    pub fn with_tags(mut self, tags: Vec<String>) -> Self {
        self.tags = tags;
        self
    }

    pub fn matches_filter(&self, filter: &RecordFilter) -> bool {
        if let Some(ref cat) = filter.category {
            if &self.category != cat {
                return false;
            }
        }
        if let Some(min) = filter.min_value {
            if self.value < min {
                return false;
            }
        }
        if let Some(max) = filter.max_value {
            if self.value > max {
                return false;
            }
        }
        if let Some(ref tags) = filter.required_tags {
            for tag in tags {
                if !self.tags.contains(tag) {
                    return false;
                }
            }
        }
        true
    }
}

#[derive(Debug, Default)]
pub struct RecordFilter {
    pub category: Option<Category>,
    pub min_value: Option<f64>,
    pub max_value: Option<f64>,
    pub required_tags: Option<Vec<String>>,
}

impl RecordFilter {
    pub fn new() -> Self {
        RecordFilter::default()
    }

    pub fn with_category(mut self, cat: Category) -> Self {
        self.category = Some(cat);
        self
    }

    pub fn with_value_range(mut self, min: f64, max: f64) -> Self {
        self.min_value = Some(min);
        self.max_value = Some(max);
        self
    }
}

// ── Aggregation ─────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct AggregateStats {
    pub count: usize,
    pub sum: f64,
    pub min: f64,
    pub max: f64,
}

impl AggregateStats {
    pub fn new() -> Self {
        AggregateStats {
            count: 0,
            sum: 0.0,
            min: f64::INFINITY,
            max: f64::NEG_INFINITY,
        }
    }

    pub fn add(&mut self, value: f64) {
        self.count += 1;
        self.sum += value;
        if value < self.min {
            self.min = value;
        }
        if value > self.max {
            self.max = value;
        }
    }

    pub fn avg(&self) -> f64 {
        if self.count == 0 {
            0.0
        } else {
            self.sum / self.count as f64
        }
    }

    pub fn merge(&mut self, other: &AggregateStats) {
        self.count += other.count;
        self.sum += other.sum;
        if other.min < self.min {
            self.min = other.min;
        }
        if other.max > self.max {
            self.max = other.max;
        }
    }
}

impl fmt::Display for AggregateStats {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "count={}, sum={:.2}, avg={:.2}, min={:.2}, max={:.2}",
            self.count,
            self.sum,
            self.avg(),
            self.min,
            self.max
        )
    }
}

// ── RecordStore ─────────────────────────────────────────────

pub struct RecordStore {
    records: BTreeMap<u64, Record>,
    by_category: HashMap<Category, Vec<u64>>,
    by_tag: HashMap<String, Vec<u64>>,
    next_id: u64,
}

impl RecordStore {
    pub fn new() -> Self {
        RecordStore {
            records: BTreeMap::new(),
            by_category: HashMap::new(),
            by_tag: HashMap::new(),
            next_id: 1,
        }
    }

    pub fn insert(&mut self, mut record: Record) -> u64 {
        if record.id == 0 {
            record.id = self.next_id;
            self.next_id += 1;
        }
        let id = record.id;
        self.by_category
            .entry(record.category.clone())
            .or_default()
            .push(id);
        for tag in &record.tags {
            self.by_tag.entry(tag.clone()).or_default().push(id);
        }
        self.records.insert(id, record);
        id
    }

    pub fn get(&self, id: u64) -> Option<&Record> {
        self.records.get(&id)
    }

    pub fn remove(&mut self, id: u64) -> Option<Record> {
        let record = self.records.remove(&id)?;
        if let Some(ids) = self.by_category.get_mut(&record.category) {
            ids.retain(|&x| x != id);
        }
        for tag in &record.tags {
            if let Some(ids) = self.by_tag.get_mut(tag) {
                ids.retain(|&x| x != id);
            }
        }
        Some(record)
    }

    pub fn query(&self, filter: &RecordFilter) -> Vec<&Record> {
        let candidate_ids: Box<dyn Iterator<Item = &u64>> =
            if let Some(ref cat) = filter.category {
                match self.by_category.get(cat) {
                    Some(ids) => Box::new(ids.iter()),
                    None => return Vec::new(),
                }
            } else {
                Box::new(self.records.keys())
            };

        let mut results: Vec<&Record> = candidate_ids
            .filter_map(|id| self.records.get(id))
            .filter(|r| r.matches_filter(filter))
            .collect();

        results.sort_by(|a, b| b.value.partial_cmp(&a.value).unwrap_or(std::cmp::Ordering::Equal));
        results
    }

    pub fn aggregate_by_category(&self) -> HashMap<Category, AggregateStats> {
        let mut stats: HashMap<Category, AggregateStats> = HashMap::new();
        for record in self.records.values() {
            stats
                .entry(record.category.clone())
                .or_insert_with(AggregateStats::new)
                .add(record.value);
        }
        stats
    }

    pub fn len(&self) -> usize {
        self.records.len()
    }

    pub fn is_empty(&self) -> bool {
        self.records.is_empty()
    }

    pub fn iter(&self) -> impl Iterator<Item = &Record> {
        self.records.values()
    }
}

// ── Graph ───────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct Edge {
    pub to: usize,
    pub weight: f64,
}

pub struct Graph {
    adj: Vec<Vec<Edge>>,
    node_count: usize,
    directed: bool,
}

impl Graph {
    pub fn new(node_count: usize, directed: bool) -> Self {
        Graph {
            adj: vec![Vec::new(); node_count],
            node_count,
            directed,
        }
    }

    pub fn add_edge(&mut self, from: usize, to: usize, weight: f64) {
        self.adj[from].push(Edge { to, weight });
        if !self.directed {
            self.adj[to].push(Edge { to: from, weight });
        }
    }

    pub fn neighbors(&self, node: usize) -> &[Edge] {
        &self.adj[node]
    }

    pub fn bfs(&self, start: usize) -> Vec<usize> {
        let mut visited = vec![false; self.node_count];
        let mut queue = VecDeque::new();
        let mut order = Vec::new();

        visited[start] = true;
        queue.push_back(start);

        while let Some(node) = queue.pop_front() {
            order.push(node);
            for edge in &self.adj[node] {
                if !visited[edge.to] {
                    visited[edge.to] = true;
                    queue.push_back(edge.to);
                }
            }
        }
        order
    }

    pub fn dfs(&self, start: usize) -> Vec<usize> {
        let mut visited = vec![false; self.node_count];
        let mut order = Vec::new();
        self.dfs_visit(start, &mut visited, &mut order);
        order
    }

    fn dfs_visit(&self, node: usize, visited: &mut Vec<bool>, order: &mut Vec<usize>) {
        if visited[node] {
            return;
        }
        visited[node] = true;
        order.push(node);
        for edge in &self.adj[node] {
            self.dfs_visit(edge.to, visited, order);
        }
    }

    pub fn dijkstra(&self, start: usize) -> Vec<f64> {
        let mut dist = vec![f64::INFINITY; self.node_count];
        let mut visited = vec![false; self.node_count];
        dist[start] = 0.0;

        for _ in 0..self.node_count {
            let mut u = None;
            let mut min_dist = f64::INFINITY;
            for v in 0..self.node_count {
                if !visited[v] && dist[v] < min_dist {
                    min_dist = dist[v];
                    u = Some(v);
                }
            }
            let u = match u {
                Some(v) => v,
                None => break,
            };
            visited[u] = true;
            for edge in &self.adj[u] {
                let new_dist = dist[u] + edge.weight;
                if new_dist < dist[edge.to] {
                    dist[edge.to] = new_dist;
                }
            }
        }
        dist
    }

    pub fn has_cycle(&self) -> bool {
        if self.directed {
            self.has_cycle_directed()
        } else {
            self.has_cycle_undirected()
        }
    }

    fn has_cycle_directed(&self) -> bool {
        let mut color = vec![0u8; self.node_count]; // 0=white, 1=gray, 2=black
        for start in 0..self.node_count {
            if color[start] == 0 && self.dfs_cycle(start, &mut color) {
                return true;
            }
        }
        false
    }

    fn dfs_cycle(&self, node: usize, color: &mut Vec<u8>) -> bool {
        color[node] = 1;
        for edge in &self.adj[node] {
            match color[edge.to] {
                1 => return true,
                0 => {
                    if self.dfs_cycle(edge.to, color) {
                        return true;
                    }
                }
                _ => {}
            }
        }
        color[node] = 2;
        false
    }

    fn has_cycle_undirected(&self) -> bool {
        let mut visited = vec![false; self.node_count];
        for start in 0..self.node_count {
            if !visited[start] {
                if self.dfs_cycle_undirected(start, usize::MAX, &mut visited) {
                    return true;
                }
            }
        }
        false
    }

    fn dfs_cycle_undirected(
        &self,
        node: usize,
        parent: usize,
        visited: &mut Vec<bool>,
    ) -> bool {
        visited[node] = true;
        for edge in &self.adj[node] {
            if !visited[edge.to] {
                if self.dfs_cycle_undirected(edge.to, node, visited) {
                    return true;
                }
            } else if edge.to != parent {
                return true;
            }
        }
        false
    }

    pub fn topological_sort(&self) -> Result<Vec<usize>> {
        if !self.directed {
            return Err(AppError::InvalidInput(
                "Topological sort requires a directed graph".to_string(),
            ));
        }
        let mut in_degree = vec![0usize; self.node_count];
        for edges in &self.adj {
            for edge in edges {
                in_degree[edge.to] += 1;
            }
        }
        let mut queue: VecDeque<usize> = (0..self.node_count)
            .filter(|&n| in_degree[n] == 0)
            .collect();
        let mut order = Vec::with_capacity(self.node_count);

        while let Some(node) = queue.pop_front() {
            order.push(node);
            for edge in &self.adj[node] {
                in_degree[edge.to] -= 1;
                if in_degree[edge.to] == 0 {
                    queue.push_back(edge.to);
                }
            }
        }

        if order.len() != self.node_count {
            Err(AppError::InvalidInput("Graph contains a cycle".to_string()))
        } else {
            Ok(order)
        }
    }
}

// ── Text Processing ─────────────────────────────────────────

pub struct TokenStats {
    pub total_tokens: usize,
    pub unique_tokens: usize,
    pub frequencies: HashMap<String, usize>,
}

pub fn tokenize(text: &str) -> Vec<String> {
    let mut tokens = Vec::new();
    let mut current = String::new();
    for ch in text.chars() {
        if ch.is_alphanumeric() || ch == '_' {
            current.push(ch.to_ascii_lowercase());
        } else if !current.is_empty() {
            tokens.push(std::mem::take(&mut current));
        }
    }
    if !current.is_empty() {
        tokens.push(current);
    }
    tokens
}

pub fn word_frequencies(text: &str) -> TokenStats {
    let tokens = tokenize(text);
    let mut frequencies: HashMap<String, usize> = HashMap::new();
    for token in &tokens {
        *frequencies.entry(token.clone()).or_insert(0) += 1;
    }
    TokenStats {
        total_tokens: tokens.len(),
        unique_tokens: frequencies.len(),
        frequencies,
    }
}

pub fn cosine_similarity(a: &HashMap<String, usize>, b: &HashMap<String, usize>) -> f64 {
    let all_keys: HashSet<&String> = a.keys().chain(b.keys()).collect();
    let mut dot = 0.0;
    let mut mag_a = 0.0;
    let mut mag_b = 0.0;
    for key in all_keys {
        let va = *a.get(key).unwrap_or(&0) as f64;
        let vb = *b.get(key).unwrap_or(&0) as f64;
        dot += va * vb;
        mag_a += va * va;
        mag_b += vb * vb;
    }
    let denom = mag_a.sqrt() * mag_b.sqrt();
    if denom == 0.0 {
        0.0
    } else {
        dot / denom
    }
}

// ── Matrix ──────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct Matrix {
    data: Vec<f64>,
    rows: usize,
    cols: usize,
}

impl Matrix {
    pub fn zeros(rows: usize, cols: usize) -> Self {
        Matrix {
            data: vec![0.0; rows * cols],
            rows,
            cols,
        }
    }

    pub fn identity(size: usize) -> Self {
        let mut m = Self::zeros(size, size);
        for i in 0..size {
            m.data[i * size + i] = 1.0;
        }
        m
    }

    pub fn get(&self, row: usize, col: usize) -> f64 {
        self.data[row * self.cols + col]
    }

    pub fn set(&mut self, row: usize, col: usize, value: f64) {
        self.data[row * self.cols + col] = value;
    }

    pub fn multiply(&self, other: &Matrix) -> Result<Matrix> {
        if self.cols != other.rows {
            return Err(AppError::InvalidInput(format!(
                "Cannot multiply {}x{} by {}x{}",
                self.rows, self.cols, other.rows, other.cols
            )));
        }
        let mut result = Matrix::zeros(self.rows, other.cols);
        for i in 0..self.rows {
            for j in 0..other.cols {
                let mut sum = 0.0;
                for k in 0..self.cols {
                    sum += self.get(i, k) * other.get(k, j);
                }
                result.set(i, j, sum);
            }
        }
        Ok(result)
    }

    pub fn transpose(&self) -> Matrix {
        let mut result = Matrix::zeros(self.cols, self.rows);
        for i in 0..self.rows {
            for j in 0..self.cols {
                result.set(j, i, self.get(i, j));
            }
        }
        result
    }

    pub fn determinant(&self) -> Result<f64> {
        if self.rows != self.cols {
            return Err(AppError::InvalidInput("Matrix must be square".to_string()));
        }
        if self.rows == 1 {
            return Ok(self.data[0]);
        }
        if self.rows == 2 {
            return Ok(self.get(0, 0) * self.get(1, 1) - self.get(0, 1) * self.get(1, 0));
        }
        let mut det = 0.0;
        for j in 0..self.cols {
            let minor = self.minor(0, j);
            let sign = if j % 2 == 0 { 1.0 } else { -1.0 };
            det += sign * self.get(0, j) * minor.determinant()?;
        }
        Ok(det)
    }

    fn minor(&self, skip_row: usize, skip_col: usize) -> Matrix {
        let size = self.rows - 1;
        let mut result = Matrix::zeros(size, size);
        let mut ri = 0;
        for i in 0..self.rows {
            if i == skip_row {
                continue;
            }
            let mut ci = 0;
            for j in 0..self.cols {
                if j == skip_col {
                    continue;
                }
                result.set(ri, ci, self.get(i, j));
                ci += 1;
            }
            ri += 1;
        }
        result
    }
}

impl fmt::Display for Matrix {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for i in 0..self.rows {
            write!(f, "[")?;
            for j in 0..self.cols {
                if j > 0 {
                    write!(f, ", ")?;
                }
                write!(f, "{:.2}", self.get(i, j))?;
            }
            writeln!(f, "]")?;
        }
        Ok(())
    }
}

// ── CSV Parser ──────────────────────────────────────────────

pub struct CsvRow {
    pub fields: Vec<String>,
}

impl CsvRow {
    pub fn get(&self, index: usize) -> Option<&str> {
        self.fields.get(index).map(|s| s.as_str())
    }

    pub fn get_f64(&self, index: usize) -> Result<f64> {
        self.fields
            .get(index)
            .ok_or_else(|| AppError::NotFound(format!("Column {}", index)))?
            .parse::<f64>()
            .map_err(|e| AppError::Parse(e.to_string()))
    }
}

pub fn parse_csv_line(line: &str) -> CsvRow {
    let mut fields = Vec::new();
    let mut current = String::new();
    let mut in_quotes = false;

    for ch in line.chars() {
        match ch {
            '"' => in_quotes = !in_quotes,
            ',' if !in_quotes => {
                fields.push(std::mem::take(&mut current));
            }
            _ => current.push(ch),
        }
    }
    fields.push(current);
    CsvRow { fields }
}

pub fn read_csv(path: &Path) -> Result<Vec<CsvRow>> {
    let file = File::open(path)?;
    let reader = BufReader::new(file);
    let mut rows = Vec::new();
    for line_result in reader.lines() {
        let line = line_result?;
        if !line.is_empty() {
            rows.push(parse_csv_line(&line));
        }
    }
    Ok(rows)
}

// ── Pipeline ────────────────────────────────────────────────

pub trait Stage: Send + Sync {
    fn name(&self) -> &str;
    fn process(&self, records: Vec<Record>) -> Vec<Record>;
}

pub struct FilterStage {
    filter: RecordFilter,
}

impl FilterStage {
    pub fn new(filter: RecordFilter) -> Self {
        FilterStage { filter }
    }
}

impl Stage for FilterStage {
    fn name(&self) -> &str {
        "filter"
    }

    fn process(&self, records: Vec<Record>) -> Vec<Record> {
        records
            .into_iter()
            .filter(|r| r.matches_filter(&self.filter))
            .collect()
    }
}

pub struct SortStage {
    descending: bool,
}

impl Stage for SortStage {
    fn name(&self) -> &str {
        "sort"
    }

    fn process(&self, mut records: Vec<Record>) -> Vec<Record> {
        if self.descending {
            records.sort_by(|a, b| b.value.partial_cmp(&a.value).unwrap_or(std::cmp::Ordering::Equal));
        } else {
            records.sort_by(|a, b| a.value.partial_cmp(&b.value).unwrap_or(std::cmp::Ordering::Equal));
        }
        records
    }
}

pub struct Pipeline {
    stages: Vec<Box<dyn Stage>>,
}

impl Pipeline {
    pub fn new() -> Self {
        Pipeline {
            stages: Vec::new(),
        }
    }

    pub fn add_stage(mut self, stage: Box<dyn Stage>) -> Self {
        self.stages.push(stage);
        self
    }

    pub fn process(&self, records: Vec<Record>) -> Vec<Record> {
        let mut result = records;
        for stage in &self.stages {
            let start = Instant::now();
            result = stage.process(result);
            let elapsed = start.elapsed();
            eprintln!(
                "Stage '{}': {} records in {:.3}ms",
                stage.name(),
                result.len(),
                elapsed.as_secs_f64() * 1000.0
            );
        }
        result
    }

    pub fn process_batched(&self, records: Vec<Record>, batch_size: usize) -> Vec<Vec<Record>> {
        records
            .chunks(batch_size)
            .map(|chunk| self.process(chunk.to_vec()))
            .collect()
    }
}

// ── Main ────────────────────────────────────────────────────

fn generate_test_records(count: u64) -> Vec<Record> {
    let categories = Category::all();
    let tag_pool = vec![
        "important", "reviewed", "pending", "archived", "flagged",
    ];
    let mut records = Vec::with_capacity(count as usize);
    for i in 1..=count {
        let cat = categories[(i as usize) % categories.len()].clone();
        let tags: Vec<String> = tag_pool
            .iter()
            .enumerate()
            .filter(|(j, _)| (i as usize + j) % 3 == 0)
            .map(|(_, t)| t.to_string())
            .collect();
        let value = ((i * 7 + 13) % 1000) as f64 + (i as f64 * 0.37).fract() * 100.0;
        let record = Record::new(i, format!("record_{:06}", i), cat, value).with_tags(tags);
        records.push(record);
    }
    records
}

fn main() -> Result<()> {
    let config = Config::default()
        .with_cache_size(512)
        .with_batch_size(50);

    let errors = config.validate();
    if !errors.is_empty() {
        for err in &errors {
            eprintln!("Config error: {}", err);
        }
        return Err(AppError::InvalidInput("Invalid configuration".to_string()));
    }

    let mut store = RecordStore::new();
    let records = generate_test_records(500);
    for record in records {
        store.insert(record);
    }
    println!("Store has {} records", store.len());

    let filter = RecordFilter::new()
        .with_category(Category::Alpha)
        .with_value_range(100.0, 500.0);
    let results = store.query(&filter);
    println!("Query returned {} records", results.len());

    let stats = store.aggregate_by_category();
    for (cat, stat) in &stats {
        println!("  {}: {}", cat, stat);
    }

    let pipeline = Pipeline::new()
        .add_stage(Box::new(FilterStage::new(
            RecordFilter::new().with_value_range(50.0, 900.0),
        )))
        .add_stage(Box::new(SortStage { descending: true }));

    let all_records: Vec<Record> = store.iter().cloned().collect();
    let processed = pipeline.process(all_records);
    println!("Pipeline processed {} records", processed.len());

    let mut graph = Graph::new(6, true);
    graph.add_edge(0, 1, 4.0);
    graph.add_edge(0, 2, 2.0);
    graph.add_edge(1, 3, 3.0);
    graph.add_edge(2, 1, 1.0);
    graph.add_edge(2, 3, 5.0);
    graph.add_edge(3, 4, 1.0);
    graph.add_edge(1, 4, 6.0);
    graph.add_edge(4, 5, 2.0);

    let distances = graph.dijkstra(0);
    for (i, d) in distances.iter().enumerate() {
        println!("Distance 0 -> {}: {:.1}", i, d);
    }

    let topo = graph.topological_sort()?;
    println!("Topological order: {:?}", topo);

    let text = "The quick brown fox jumps over the lazy dog and the fox";
    let stats = word_frequencies(text);
    println!(
        "Text stats: {} total, {} unique tokens",
        stats.total_tokens, stats.unique_tokens
    );

    let mut m = Matrix::zeros(3, 3);
    m.set(0, 0, 1.0);
    m.set(0, 1, 2.0);
    m.set(0, 2, 3.0);
    m.set(1, 0, 4.0);
    m.set(1, 1, 5.0);
    m.set(1, 2, 6.0);
    m.set(2, 0, 7.0);
    m.set(2, 1, 8.0);
    m.set(2, 2, 9.0);
    let det = m.determinant()?;
    println!("Determinant: {:.2}", det);

    let transposed = m.transpose();
    println!("Transposed:\n{}", transposed);

    Ok(())
}
