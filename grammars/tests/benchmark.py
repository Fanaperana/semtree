"""
Benchmark Python file: data processing pipeline with typed classes,
decorators, generators, context managers, and comprehensions.
"""
from __future__ import annotations
import os
import sys
import json
import time
import hashlib
import logging
from typing import Any, Dict, List, Optional, Tuple, Iterator, Callable, TypeVar, Generic
from dataclasses import dataclass, field
from contextlib import contextmanager
from functools import wraps, lru_cache
from collections import defaultdict, Counter
from pathlib import Path

T = TypeVar("T")
K = TypeVar("K")
V = TypeVar("V")

logger = logging.getLogger(__name__)


def timer(func: Callable) -> Callable:
    @wraps(func)
    def wrapper(*args, **kwargs):
        start = time.perf_counter()
        result = func(*args, **kwargs)
        elapsed = time.perf_counter() - start
        logger.info(f"{func.__name__} took {elapsed:.4f}s")
        return result
    return wrapper


def retry(max_attempts: int = 3, delay: float = 1.0):
    def decorator(func: Callable) -> Callable:
        @wraps(func)
        def wrapper(*args, **kwargs):
            last_error = None
            for attempt in range(1, max_attempts + 1):
                try:
                    return func(*args, **kwargs)
                except Exception as e:
                    last_error = e
                    if attempt < max_attempts:
                        time.sleep(delay * attempt)
                        logger.warning(f"Retry {attempt}/{max_attempts} for {func.__name__}")
            raise last_error
        return wrapper
    return decorator


def validate_types(**type_specs):
    def decorator(func: Callable) -> Callable:
        @wraps(func)
        def wrapper(*args, **kwargs):
            params = func.__code__.co_varnames[:func.__code__.co_argcount]
            for i, (param, arg) in enumerate(zip(params, args)):
                if param in type_specs:
                    expected = type_specs[param]
                    if not isinstance(arg, expected):
                        raise TypeError(
                            f"Argument '{param}' expected {expected.__name__}, got {type(arg).__name__}"
                        )
            return func(*args, **kwargs)
        return wrapper
    return decorator


@contextmanager
def temporary_directory(prefix: str = "tmp_"):
    import tempfile
    import shutil
    path = tempfile.mkdtemp(prefix=prefix)
    try:
        yield Path(path)
    finally:
        shutil.rmtree(path, ignore_errors=True)


@contextmanager
def file_transaction(filepath: Path):
    temp_path = filepath.with_suffix(".tmp")
    try:
        yield temp_path
        if temp_path.exists():
            temp_path.rename(filepath)
    except Exception:
        if temp_path.exists():
            temp_path.unlink()
        raise


@dataclass
class Config:
    database_url: str = "sqlite:///data.db"
    cache_size: int = 1024
    batch_size: int = 100
    max_retries: int = 3
    log_level: str = "INFO"
    output_dir: Path = field(default_factory=lambda: Path("output"))
    tags: Dict[str, str] = field(default_factory=dict)

    def validate(self) -> List[str]:
        errors = []
        if self.cache_size <= 0:
            errors.append("cache_size must be positive")
        if self.batch_size <= 0:
            errors.append("batch_size must be positive")
        if self.log_level not in ("DEBUG", "INFO", "WARNING", "ERROR"):
            errors.append(f"Invalid log_level: {self.log_level}")
        return errors

    @classmethod
    def from_dict(cls, data: Dict[str, Any]) -> Config:
        if "output_dir" in data:
            data["output_dir"] = Path(data["output_dir"])
        return cls(**data)

    @classmethod
    def from_json(cls, path: Path) -> Config:
        with open(path) as f:
            return cls.from_dict(json.load(f))

    def to_dict(self) -> Dict[str, Any]:
        result = {}
        result["database_url"] = self.database_url
        result["cache_size"] = self.cache_size
        result["batch_size"] = self.batch_size
        result["max_retries"] = self.max_retries
        result["log_level"] = self.log_level
        result["output_dir"] = str(self.output_dir)
        result["tags"] = dict(self.tags)
        return result


@dataclass
class Record:
    id: int
    name: str
    category: str
    value: float
    tags: List[str] = field(default_factory=list)
    metadata: Dict[str, Any] = field(default_factory=dict)
    timestamp: float = field(default_factory=time.time)

    @property
    def checksum(self) -> str:
        raw = f"{self.id}:{self.name}:{self.value}"
        return hashlib.sha256(raw.encode()).hexdigest()[:16]

    def matches_filter(self, category: Optional[str] = None,
                       min_value: Optional[float] = None,
                       max_value: Optional[float] = None,
                       required_tags: Optional[List[str]] = None) -> bool:
        if category is not None and self.category != category:
            return False
        if min_value is not None and self.value < min_value:
            return False
        if max_value is not None and self.value > max_value:
            return False
        if required_tags is not None:
            for tag in required_tags:
                if tag not in self.tags:
                    return False
        return True

    def to_dict(self) -> Dict[str, Any]:
        return {
            "id": self.id,
            "name": self.name,
            "category": self.category,
            "value": self.value,
            "tags": list(self.tags),
            "metadata": dict(self.metadata),
            "timestamp": self.timestamp,
            "checksum": self.checksum,
        }


class LRUCache(Generic[K, V]):
    def __init__(self, capacity: int):
        self._capacity = capacity
        self._cache: Dict[K, V] = {}
        self._order: List[K] = []

    def get(self, key: K, default: Optional[V] = None) -> Optional[V]:
        if key not in self._cache:
            return default
        self._order.remove(key)
        self._order.append(key)
        return self._cache[key]

    def put(self, key: K, value: V) -> None:
        if key in self._cache:
            self._order.remove(key)
        elif len(self._cache) >= self._capacity:
            evicted = self._order.pop(0)
            del self._cache[evicted]
        self._cache[key] = value
        self._order.append(key)

    def __contains__(self, key: K) -> bool:
        return key in self._cache

    def __len__(self) -> int:
        return len(self._cache)

    def clear(self) -> None:
        self._cache.clear()
        self._order.clear()

    @property
    def keys(self) -> List[K]:
        return list(self._order)


class DataPipeline:
    def __init__(self, config: Config):
        self.config = config
        self._stages: List[Callable] = []
        self._cache: LRUCache[str, Any] = LRUCache(config.cache_size)
        self._stats: Dict[str, int] = defaultdict(int)

    def add_stage(self, name: str, fn: Callable) -> "DataPipeline":
        self._stages.append((name, fn))
        return self

    @timer
    def process(self, records: List[Record]) -> List[Record]:
        result = list(records)
        for stage_name, stage_fn in self._stages:
            start = time.perf_counter()
            result = stage_fn(result)
            elapsed = time.perf_counter() - start
            self._stats[stage_name] += 1
            logger.debug(f"Stage '{stage_name}': {len(result)} records in {elapsed:.3f}s")
        return result

    def process_batched(self, records: List[Record]) -> Iterator[List[Record]]:
        batch_size = self.config.batch_size
        for i in range(0, len(records), batch_size):
            batch = records[i:i + batch_size]
            yield self.process(batch)

    def get_stats(self) -> Dict[str, int]:
        return dict(self._stats)


class RecordStore:
    def __init__(self):
        self._records: Dict[int, Record] = {}
        self._by_category: Dict[str, List[int]] = defaultdict(list)
        self._by_tag: Dict[str, List[int]] = defaultdict(list)
        self._next_id: int = 1

    def add(self, record: Record) -> int:
        if record.id == 0:
            record.id = self._next_id
            self._next_id += 1
        self._records[record.id] = record
        self._by_category[record.category].append(record.id)
        for tag in record.tags:
            self._by_tag[tag].append(record.id)
        return record.id

    def get(self, record_id: int) -> Optional[Record]:
        return self._records.get(record_id)

    def delete(self, record_id: int) -> bool:
        record = self._records.pop(record_id, None)
        if record is None:
            return False
        cat_list = self._by_category.get(record.category, [])
        if record_id in cat_list:
            cat_list.remove(record_id)
        for tag in record.tags:
            tag_list = self._by_tag.get(tag, [])
            if record_id in tag_list:
                tag_list.remove(record_id)
        return True

    def query(self, category: Optional[str] = None,
              tags: Optional[List[str]] = None,
              min_value: Optional[float] = None,
              max_value: Optional[float] = None) -> List[Record]:
        if category is not None:
            ids = set(self._by_category.get(category, []))
        else:
            ids = set(self._records.keys())

        if tags is not None:
            for tag in tags:
                tag_ids = set(self._by_tag.get(tag, []))
                ids = ids & tag_ids

        results = []
        for rid in ids:
            record = self._records[rid]
            if min_value is not None and record.value < min_value:
                continue
            if max_value is not None and record.value > max_value:
                continue
            results.append(record)

        return sorted(results, key=lambda r: r.value, reverse=True)

    def aggregate(self, group_by: str = "category") -> Dict[str, Dict[str, float]]:
        groups: Dict[str, List[float]] = defaultdict(list)
        for record in self._records.values():
            key = getattr(record, group_by, "unknown")
            groups[key].append(record.value)

        result = {}
        for key, values in groups.items():
            total = sum(values)
            count = len(values)
            result[key] = {
                "count": count,
                "sum": total,
                "avg": total / count if count > 0 else 0.0,
                "min": min(values) if values else 0.0,
                "max": max(values) if values else 0.0,
            }
        return result

    def __len__(self) -> int:
        return len(self._records)

    def __iter__(self) -> Iterator[Record]:
        yield from self._records.values()

    def export_json(self, path: Path) -> int:
        data = [r.to_dict() for r in self._records.values()]
        with open(path, "w") as f:
            json.dump(data, f, indent=2)
        return len(data)

    @classmethod
    def import_json(cls, path: Path) -> "RecordStore":
        store = cls()
        with open(path) as f:
            data = json.load(f)
        for item in data:
            record = Record(
                id=item["id"],
                name=item["name"],
                category=item["category"],
                value=item["value"],
                tags=item.get("tags", []),
                metadata=item.get("metadata", {}),
                timestamp=item.get("timestamp", time.time()),
            )
            store.add(record)
        return store


class TextAnalyzer:
    def __init__(self, stop_words: Optional[List[str]] = None):
        self.stop_words = set(stop_words or [
            "the", "a", "an", "is", "are", "was", "were", "be", "been",
            "being", "have", "has", "had", "do", "does", "did", "will",
            "would", "could", "should", "may", "might", "shall", "can",
            "to", "of", "in", "for", "on", "with", "at", "by", "from",
            "as", "into", "through", "during", "before", "after", "and",
            "but", "or", "nor", "not", "so", "yet", "both", "either",
        ])

    def tokenize(self, text: str) -> List[str]:
        tokens = []
        current = []
        for ch in text.lower():
            if ch.isalnum():
                current.append(ch)
            elif current:
                tokens.append("".join(current))
                current = []
        if current:
            tokens.append("".join(current))
        return tokens

    def word_frequencies(self, text: str) -> Counter:
        tokens = self.tokenize(text)
        filtered = [t for t in tokens if t not in self.stop_words and len(t) > 1]
        return Counter(filtered)

    def ngrams(self, text: str, n: int = 2) -> List[Tuple[str, ...]]:
        tokens = self.tokenize(text)
        if len(tokens) < n:
            return []
        return [tuple(tokens[i:i + n]) for i in range(len(tokens) - n + 1)]

    def similarity(self, text_a: str, text_b: str) -> float:
        freq_a = self.word_frequencies(text_a)
        freq_b = self.word_frequencies(text_b)
        all_words = set(freq_a.keys()) | set(freq_b.keys())
        if not all_words:
            return 0.0
        dot_product = sum(freq_a.get(w, 0) * freq_b.get(w, 0) for w in all_words)
        mag_a = sum(v ** 2 for v in freq_a.values()) ** 0.5
        mag_b = sum(v ** 2 for v in freq_b.values()) ** 0.5
        if mag_a == 0 or mag_b == 0:
            return 0.0
        return dot_product / (mag_a * mag_b)

    @timer
    def analyze_corpus(self, documents: List[str]) -> Dict[str, Any]:
        all_freqs = Counter()
        doc_count = len(documents)
        doc_freqs = Counter()
        for doc in documents:
            freqs = self.word_frequencies(doc)
            all_freqs.update(freqs)
            for word in set(freqs.keys()):
                doc_freqs[word] += 1
        idf = {
            word: (doc_count / count) for word, count in doc_freqs.items()
        }
        top_words = all_freqs.most_common(50)
        return {
            "total_documents": doc_count,
            "unique_words": len(all_freqs),
            "top_words": top_words,
            "idf_scores": dict(sorted(idf.items(), key=lambda x: x[1], reverse=True)[:50]),
        }


def generate_records(count: int, categories: Optional[List[str]] = None) -> Iterator[Record]:
    cats = categories or ["alpha", "beta", "gamma", "delta", "epsilon"]
    tags_pool = ["important", "reviewed", "pending", "archived", "flagged", "new", "updated"]
    import random
    for i in range(1, count + 1):
        cat = cats[i % len(cats)]
        num_tags = random.randint(0, 3)
        selected_tags = random.sample(tags_pool, min(num_tags, len(tags_pool)))
        yield Record(
            id=i,
            name=f"record_{i:06d}",
            category=cat,
            value=round(random.uniform(0.0, 1000.0), 2),
            tags=selected_tags,
            metadata={"source": "generated", "batch": i // 100},
        )


@lru_cache(maxsize=128)
def fibonacci(n: int) -> int:
    if n <= 1:
        return n
    return fibonacci(n - 1) + fibonacci(n - 2)


def matrix_multiply(a: List[List[float]], b: List[List[float]]) -> List[List[float]]:
    rows_a = len(a)
    cols_a = len(a[0])
    cols_b = len(b[0])
    result = [[0.0] * cols_b for _ in range(rows_a)]
    for i in range(rows_a):
        for j in range(cols_b):
            total = 0.0
            for k in range(cols_a):
                total += a[i][k] * b[k][j]
            result[i][j] = total
    return result


class Graph(Generic[T]):
    def __init__(self, directed: bool = False):
        self._adj: Dict[T, List[Tuple[T, float]]] = defaultdict(list)
        self._directed = directed

    def add_edge(self, u: T, v: T, weight: float = 1.0) -> None:
        self._adj[u].append((v, weight))
        if not self._directed:
            self._adj[v].append((u, weight))

    def neighbors(self, node: T) -> List[Tuple[T, float]]:
        return list(self._adj.get(node, []))

    def bfs(self, start: T) -> List[T]:
        visited = set()
        queue = [start]
        visited.add(start)
        order = []
        while queue:
            node = queue.pop(0)
            order.append(node)
            for neighbor, _ in self._adj.get(node, []):
                if neighbor not in visited:
                    visited.add(neighbor)
                    queue.append(neighbor)
        return order

    def dfs(self, start: T) -> List[T]:
        visited = set()
        order = []

        def visit(node: T) -> None:
            if node in visited:
                return
            visited.add(node)
            order.append(node)
            for neighbor, _ in self._adj.get(node, []):
                visit(neighbor)

        visit(start)
        return order

    def shortest_path(self, start: T, end: T) -> Tuple[float, List[T]]:
        import heapq
        dist: Dict[T, float] = {start: 0.0}
        prev: Dict[T, Optional[T]] = {start: None}
        heap = [(0.0, id(start), start)]

        while heap:
            d, _, node = heapq.heappop(heap)
            if node == end:
                path = []
                current: Optional[T] = end
                while current is not None:
                    path.append(current)
                    current = prev.get(current)
                path.reverse()
                return d, path
            if d > dist.get(node, float("inf")):
                continue
            for neighbor, weight in self._adj.get(node, []):
                new_dist = d + weight
                if new_dist < dist.get(neighbor, float("inf")):
                    dist[neighbor] = new_dist
                    prev[neighbor] = node
                    heapq.heappush(heap, (new_dist, id(neighbor), neighbor))

        return float("inf"), []

    @property
    def nodes(self) -> List[T]:
        return list(self._adj.keys())

    @property
    def edge_count(self) -> int:
        total = sum(len(edges) for edges in self._adj.values())
        if not self._directed:
            total //= 2
        return total


def main():
    config = Config(
        cache_size=512,
        batch_size=50,
        log_level="DEBUG",
        output_dir=Path("benchmark_output"),
    )
    errors = config.validate()
    if errors:
        for err in errors:
            print(f"Config error: {err}")
        sys.exit(1)

    store = RecordStore()
    for record in generate_records(500):
        store.add(record)

    pipeline = DataPipeline(config)
    pipeline.add_stage("filter_high_value", lambda records: [
        r for r in records if r.value > 100.0
    ])
    pipeline.add_stage("sort_by_value", lambda records: sorted(
        records, key=lambda r: r.value, reverse=True
    ))
    pipeline.add_stage("tag_processed", lambda records: [
        Record(
            id=r.id, name=r.name, category=r.category,
            value=r.value, tags=r.tags + ["processed"],
            metadata={**r.metadata, "processed": True},
        )
        for r in records
    ])

    all_records = list(store)
    processed = pipeline.process(all_records)
    print(f"Processed {len(processed)} out of {len(all_records)} records")

    agg = store.aggregate("category")
    for category, stats in sorted(agg.items()):
        print(f"  {category}: count={stats['count']}, avg={stats['avg']:.2f}")

    analyzer = TextAnalyzer()
    docs = [
        "The quick brown fox jumps over the lazy dog repeatedly",
        "A fast brown fox leaps across the sleeping canine",
        "Machine learning algorithms process data efficiently",
        "Deep neural networks transform input features into predictions",
    ]
    corpus_stats = analyzer.analyze_corpus(docs)
    print(f"Corpus: {corpus_stats['unique_words']} unique words across {corpus_stats['total_documents']} docs")

    sim = analyzer.similarity(docs[0], docs[1])
    print(f"Similarity between first two docs: {sim:.4f}")

    graph = Graph[str](directed=True)
    edges = [
        ("A", "B", 4.0), ("A", "C", 2.0), ("B", "D", 3.0),
        ("C", "B", 1.0), ("C", "D", 5.0), ("D", "E", 1.0),
        ("B", "E", 6.0),
    ]
    for u, v, w in edges:
        graph.add_edge(u, v, w)

    dist, path = graph.shortest_path("A", "E")
    print(f"Shortest A->E: distance={dist}, path={' -> '.join(path)}")

    fib_values = [fibonacci(i) for i in range(30)]
    print(f"Fibonacci(29) = {fib_values[-1]}")


if __name__ == "__main__":
    main()
