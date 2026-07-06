// Event Emitter implementation
class EventEmitter {
    constructor() {
        this._events = {};
        this._maxListeners = 10;
    }

    on(event, listener) {
        if (!this._events[event]) {
            this._events[event] = [];
        }
        if (this._events[event].length >= this._maxListeners) {
            console.warn("MaxListenersExceeded for event: " + event);
        }
        this._events[event].push(listener);
        return this;
    }

    off(event, listener) {
        if (!this._events[event]) {
            return this;
        }
        const idx = this._events[event].indexOf(listener);
        if (idx !== -1) {
            this._events[event].splice(idx, 1);
        }
        return this;
    }

    emit(event, ...args) {
        if (!this._events[event]) {
            return false;
        }
        const listeners = this._events[event].slice();
        for (let i = 0; i < listeners.length; i++) {
            try {
                listeners[i].apply(this, args);
            } catch (err) {
                console.error("Error in listener for " + event, err);
            }
        }
        return true;
    }

    once(event, listener) {
        const wrapped = (...args) => {
            this.off(event, wrapped);
            listener.apply(this, args);
        };
        wrapped._original = listener;
        return this.on(event, wrapped);
    }

    removeAllListeners(event) {
        if (event) {
            delete this._events[event];
        } else {
            this._events = {};
        }
        return this;
    }

    listenerCount(event) {
        return this._events[event] ? this._events[event].length : 0;
    }
}

// Linked List data structure
class ListNode {
    constructor(value) {
        this.value = value;
        this.next = null;
        this.prev = null;
    }
}

class DoublyLinkedList {
    constructor() {
        this.head = null;
        this.tail = null;
        this.size = 0;
    }

    pushFront(value) {
        const node = new ListNode(value);
        if (this.head === null) {
            this.head = node;
            this.tail = node;
        } else {
            node.next = this.head;
            this.head.prev = node;
            this.head = node;
        }
        this.size += 1;
        return this;
    }

    pushBack(value) {
        const node = new ListNode(value);
        if (this.tail === null) {
            this.head = node;
            this.tail = node;
        } else {
            node.prev = this.tail;
            this.tail.next = node;
            this.tail = node;
        }
        this.size += 1;
        return this;
    }

    popFront() {
        if (this.head === null) {
            return undefined;
        }
        const value = this.head.value;
        this.head = this.head.next;
        if (this.head !== null) {
            this.head.prev = null;
        } else {
            this.tail = null;
        }
        this.size -= 1;
        return value;
    }

    popBack() {
        if (this.tail === null) {
            return undefined;
        }
        const value = this.tail.value;
        this.tail = this.tail.prev;
        if (this.tail !== null) {
            this.tail.next = null;
        } else {
            this.head = null;
        }
        this.size -= 1;
        return value;
    }

    find(predicate) {
        let current = this.head;
        while (current !== null) {
            if (predicate(current.value)) {
                return current.value;
            }
            current = current.next;
        }
        return undefined;
    }

    toArray() {
        const result = [];
        let current = this.head;
        while (current !== null) {
            result.push(current.value);
            current = current.next;
        }
        return result;
    }

    forEach(callback) {
        let current = this.head;
        let index = 0;
        while (current !== null) {
            callback(current.value, index);
            current = current.next;
            index += 1;
        }
    }

    insertAt(index, value) {
        if (index <= 0) {
            return this.pushFront(value);
        }
        if (index >= this.size) {
            return this.pushBack(value);
        }
        const node = new ListNode(value);
        let current = this.head;
        for (let i = 0; i < index - 1; i++) {
            current = current.next;
        }
        node.next = current.next;
        node.prev = current;
        current.next.prev = node;
        current.next = node;
        this.size += 1;
        return this;
    }

    removeAt(index) {
        if (index <= 0) {
            return this.popFront();
        }
        if (index >= this.size - 1) {
            return this.popBack();
        }
        let current = this.head;
        for (let i = 0; i < index; i++) {
            current = current.next;
        }
        current.prev.next = current.next;
        current.next.prev = current.prev;
        this.size -= 1;
        return current.value;
    }
}

// LRU Cache with Map
class LRUCache {
    constructor(capacity) {
        this.capacity = capacity;
        this.cache = new Map();
    }

    get(key) {
        if (!this.cache.has(key)) {
            return -1;
        }
        const value = this.cache.get(key);
        this.cache.delete(key);
        this.cache.set(key, value);
        return value;
    }

    put(key, value) {
        if (this.cache.has(key)) {
            this.cache.delete(key);
        } else if (this.cache.size >= this.capacity) {
            const firstKey = this.cache.keys().next().value;
            this.cache.delete(firstKey);
        }
        this.cache.set(key, value);
    }

    has(key) {
        return this.cache.has(key);
    }

    clear() {
        this.cache.clear();
    }
}

// Promise-based retry utility
async function retry(fn, maxAttempts, delay) {
    let lastError = null;
    for (let attempt = 1; attempt <= maxAttempts; attempt++) {
        try {
            const result = await fn();
            return result;
        } catch (err) {
            lastError = err;
            if (attempt < maxAttempts) {
                await sleep(delay * attempt);
            }
        }
    }
    throw lastError;
}

function sleep(ms) {
    return new Promise((resolve) => {
        setTimeout(resolve, ms);
    });
}

// Deep clone utility
function deepClone(obj) {
    if (obj === null || typeof obj !== "object") {
        return obj;
    }
    if (Array.isArray(obj)) {
        const arr = [];
        for (let i = 0; i < obj.length; i++) {
            arr.push(deepClone(obj[i]));
        }
        return arr;
    }
    const clone = {};
    const keys = Object.keys(obj);
    for (let i = 0; i < keys.length; i++) {
        clone[keys[i]] = deepClone(obj[keys[i]]);
    }
    return clone;
}

// String manipulation utilities
function capitalize(str) {
    if (str.length === 0) {
        return str;
    }
    return str.charAt(0).toUpperCase() + str.slice(1);
}

function camelToSnake(str) {
    let result = "";
    for (let i = 0; i < str.length; i++) {
        const ch = str.charAt(i);
        if (ch >= "A" && ch <= "Z") {
            if (i > 0) {
                result += "_";
            }
            result += ch.toLowerCase();
        } else {
            result += ch;
        }
    }
    return result;
}

function snakeToCamel(str) {
    const parts = str.split("_");
    let result = parts[0];
    for (let i = 1; i < parts.length; i++) {
        result += capitalize(parts[i]);
    }
    return result;
}

function truncate(str, maxLen, suffix) {
    if (str.length <= maxLen) {
        return str;
    }
    const end = suffix || "...";
    return str.slice(0, maxLen - end.length) + end;
}

function escapeHtml(str) {
    const map = {
        "&": "&amp;",
        "<": "&lt;",
        ">": "&gt;",
        "\"": "&quot;",
        "'": "&#39;"
    };
    let result = "";
    for (let i = 0; i < str.length; i++) {
        const ch = str.charAt(i);
        result += map[ch] || ch;
    }
    return result;
}

// Binary search tree
class BSTNode {
    constructor(key, value) {
        this.key = key;
        this.value = value;
        this.left = null;
        this.right = null;
        this.height = 1;
    }
}

class AVLTree {
    constructor(comparator) {
        this.root = null;
        this.comparator = comparator || ((a, b) => a - b);
        this.nodeCount = 0;
    }

    getHeight(node) {
        return node === null ? 0 : node.height;
    }

    getBalance(node) {
        return node === null ? 0 : this.getHeight(node.left) - this.getHeight(node.right);
    }

    updateHeight(node) {
        const leftH = this.getHeight(node.left);
        const rightH = this.getHeight(node.right);
        node.height = 1 + (leftH > rightH ? leftH : rightH);
    }

    rotateRight(y) {
        const x = y.left;
        const t2 = x.right;
        x.right = y;
        y.left = t2;
        this.updateHeight(y);
        this.updateHeight(x);
        return x;
    }

    rotateLeft(x) {
        const y = x.right;
        const t2 = y.left;
        y.left = x;
        x.right = t2;
        this.updateHeight(x);
        this.updateHeight(y);
        return y;
    }

    insert(key, value) {
        this.root = this._insert(this.root, key, value);
    }

    _insert(node, key, value) {
        if (node === null) {
            this.nodeCount += 1;
            return new BSTNode(key, value);
        }
        const cmp = this.comparator(key, node.key);
        if (cmp < 0) {
            node.left = this._insert(node.left, key, value);
        } else if (cmp > 0) {
            node.right = this._insert(node.right, key, value);
        } else {
            node.value = value;
            return node;
        }
        this.updateHeight(node);
        const balance = this.getBalance(node);
        if (balance > 1 && this.comparator(key, node.left.key) < 0) {
            return this.rotateRight(node);
        }
        if (balance < -1 && this.comparator(key, node.right.key) > 0) {
            return this.rotateLeft(node);
        }
        if (balance > 1 && this.comparator(key, node.left.key) > 0) {
            node.left = this.rotateLeft(node.left);
            return this.rotateRight(node);
        }
        if (balance < -1 && this.comparator(key, node.right.key) < 0) {
            node.right = this.rotateRight(node.right);
            return this.rotateLeft(node);
        }
        return node;
    }

    find(key) {
        let node = this.root;
        while (node !== null) {
            const cmp = this.comparator(key, node.key);
            if (cmp === 0) {
                return node.value;
            } else if (cmp < 0) {
                node = node.left;
            } else {
                node = node.right;
            }
        }
        return undefined;
    }

    inOrder() {
        const result = [];
        const stack = [];
        let current = this.root;
        while (current !== null || stack.length > 0) {
            while (current !== null) {
                stack.push(current);
                current = current.left;
            }
            current = stack.pop();
            result.push({ key: current.key, value: current.value });
            current = current.right;
        }
        return result;
    }

    get size() {
        return this.nodeCount;
    }
}

// HTTP Router
class Router {
    constructor() {
        this.routes = [];
        this.middleware = [];
    }

    use(fn) {
        this.middleware.push(fn);
        return this;
    }

    addRoute(method, path, handler) {
        const segments = path.split("/").filter(s => s.length > 0);
        const params = [];
        const pattern = segments.map(seg => {
            if (seg.startsWith(":")) {
                params.push(seg.slice(1));
                return "([^/]+)";
            }
            return seg;
        }).join("/");
        this.routes.push({
            method: method.toUpperCase(),
            pattern: new RegExp("^/" + pattern + "$"),
            params: params,
            handler: handler
        });
        return this;
    }

    get(path, handler) {
        return this.addRoute("GET", path, handler);
    }

    post(path, handler) {
        return this.addRoute("POST", path, handler);
    }

    put(path, handler) {
        return this.addRoute("PUT", path, handler);
    }

    delete(path, handler) {
        return this.addRoute("DELETE", path, handler);
    }

    async handle(req) {
        for (const mw of this.middleware) {
            const result = await mw(req);
            if (result !== undefined) {
                return result;
            }
        }
        for (const route of this.routes) {
            if (route.method !== req.method) {
                continue;
            }
            const match = req.url.match(route.pattern);
            if (match) {
                const params = {};
                for (let i = 0; i < route.params.length; i++) {
                    params[route.params[i]] = match[i + 1];
                }
                req.params = params;
                try {
                    return await route.handler(req);
                } catch (err) {
                    return { status: 500, body: err.message };
                }
            }
        }
        return { status: 404, body: "Not Found" };
    }
}

// Observable / reactive state
class Observable {
    constructor(initialValue) {
        this._value = initialValue;
        this._subscribers = [];
    }

    get value() {
        return this._value;
    }

    set value(newValue) {
        const oldValue = this._value;
        if (oldValue !== newValue) {
            this._value = newValue;
            this._notify(newValue, oldValue);
        }
    }

    subscribe(callback) {
        this._subscribers.push(callback);
        return () => {
            const idx = this._subscribers.indexOf(callback);
            if (idx !== -1) {
                this._subscribers.splice(idx, 1);
            }
        };
    }

    _notify(newValue, oldValue) {
        for (const sub of this._subscribers) {
            try {
                sub(newValue, oldValue);
            } catch (err) {
                console.error("Subscriber error:", err);
            }
        }
    }
}

function computed(dependencies, computeFn) {
    const result = new Observable(computeFn());
    for (const dep of dependencies) {
        dep.subscribe(() => {
            result.value = computeFn();
        });
    }
    return result;
}

// Sorted array merge
function mergeSortedArrays(a, b) {
    const result = [];
    let i = 0;
    let j = 0;
    while (i < a.length && j < b.length) {
        if (a[i] <= b[j]) {
            result.push(a[i]);
            i += 1;
        } else {
            result.push(b[j]);
            j += 1;
        }
    }
    while (i < a.length) {
        result.push(a[i]);
        i += 1;
    }
    while (j < b.length) {
        result.push(b[j]);
        j += 1;
    }
    return result;
}

// Debounce / throttle
function debounce(fn, delay) {
    let timer = null;
    return function (...args) {
        if (timer !== null) {
            clearTimeout(timer);
        }
        timer = setTimeout(() => {
            timer = null;
            fn.apply(this, args);
        }, delay);
    };
}

function throttle(fn, interval) {
    let lastCall = 0;
    let pending = null;
    return function (...args) {
        const now = Date.now();
        const remaining = interval - (now - lastCall);
        if (remaining <= 0) {
            lastCall = now;
            fn.apply(this, args);
        } else if (pending === null) {
            pending = setTimeout(() => {
                lastCall = Date.now();
                pending = null;
                fn.apply(this, args);
            }, remaining);
        }
    };
}

// Task queue with concurrency control
class TaskQueue {
    constructor(concurrency) {
        this.concurrency = concurrency || 4;
        this.running = 0;
        this.queue = [];
        this.results = [];
    }

    async add(taskFn) {
        return new Promise((resolve, reject) => {
            this.queue.push({ fn: taskFn, resolve, reject });
            this._run();
        });
    }

    async _run() {
        while (this.running < this.concurrency && this.queue.length > 0) {
            const task = this.queue.shift();
            this.running += 1;
            try {
                const result = await task.fn();
                this.results.push(result);
                task.resolve(result);
            } catch (err) {
                task.reject(err);
            } finally {
                this.running -= 1;
                this._run();
            }
        }
    }

    async drain() {
        while (this.queue.length > 0 || this.running > 0) {
            await sleep(10);
        }
        return this.results;
    }
}

// Schema validator
function validateSchema(schema, data) {
    const errors = [];

    function validate(schemaNode, value, path) {
        if (schemaNode.type === "string") {
            if (typeof value !== "string") {
                errors.push(path + ": expected string");
                return;
            }
            if (schemaNode.minLength && value.length < schemaNode.minLength) {
                errors.push(path + ": too short");
            }
            if (schemaNode.maxLength && value.length > schemaNode.maxLength) {
                errors.push(path + ": too long");
            }
            if (schemaNode.pattern) {
                const re = new RegExp(schemaNode.pattern);
                if (!re.test(value)) {
                    errors.push(path + ": pattern mismatch");
                }
            }
        } else if (schemaNode.type === "number") {
            if (typeof value !== "number") {
                errors.push(path + ": expected number");
                return;
            }
            if (schemaNode.min !== undefined && value < schemaNode.min) {
                errors.push(path + ": below minimum");
            }
            if (schemaNode.max !== undefined && value > schemaNode.max) {
                errors.push(path + ": above maximum");
            }
        } else if (schemaNode.type === "boolean") {
            if (typeof value !== "boolean") {
                errors.push(path + ": expected boolean");
            }
        } else if (schemaNode.type === "array") {
            if (!Array.isArray(value)) {
                errors.push(path + ": expected array");
                return;
            }
            if (schemaNode.items) {
                for (let i = 0; i < value.length; i++) {
                    validate(schemaNode.items, value[i], path + "[" + i + "]");
                }
            }
        } else if (schemaNode.type === "object") {
            if (typeof value !== "object" || value === null) {
                errors.push(path + ": expected object");
                return;
            }
            if (schemaNode.required) {
                for (const field of schemaNode.required) {
                    if (value[field] === undefined) {
                        errors.push(path + "." + field + ": required");
                    }
                }
            }
            if (schemaNode.properties) {
                for (const key of Object.keys(schemaNode.properties)) {
                    if (value[key] !== undefined) {
                        validate(schemaNode.properties[key], value[key], path + "." + key);
                    }
                }
            }
        }
    }

    validate(schema, data, "$");
    return errors;
}

// Matrix operations
function createMatrix(rows, cols, fill) {
    const m = [];
    for (let i = 0; i < rows; i++) {
        const row = [];
        for (let j = 0; j < cols; j++) {
            row.push(fill !== undefined ? fill : 0);
        }
        m.push(row);
    }
    return m;
}

function multiplyMatrices(a, b) {
    const rowsA = a.length;
    const colsA = a[0].length;
    const colsB = b[0].length;
    const result = createMatrix(rowsA, colsB, 0);
    for (let i = 0; i < rowsA; i++) {
        for (let j = 0; j < colsB; j++) {
            let sum = 0;
            for (let k = 0; k < colsA; k++) {
                sum += a[i][k] * b[k][j];
            }
            result[i][j] = sum;
        }
    }
    return result;
}

function transposeMatrix(m) {
    const rows = m.length;
    const cols = m[0].length;
    const result = createMatrix(cols, rows, 0);
    for (let i = 0; i < rows; i++) {
        for (let j = 0; j < cols; j++) {
            result[j][i] = m[i][j];
        }
    }
    return result;
}

export {
    EventEmitter,
    DoublyLinkedList,
    LRUCache,
    AVLTree,
    Router,
    Observable,
    TaskQueue,
    retry,
    deepClone,
    capitalize,
    camelToSnake,
    snakeToCamel,
    truncate,
    escapeHtml,
    debounce,
    throttle,
    mergeSortedArrays,
    validateSchema,
    createMatrix,
    multiplyMatrices,
    transposeMatrix
};
