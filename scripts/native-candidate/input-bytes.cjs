// Read candidate evidence through one descriptor; path checks alone cannot bound a later read.
const fs = require('node:fs');

function readBytes(file, maximum, minimum = 0) {
  const fd = fs.openSync(file, fs.constants.O_RDONLY |
    (fs.constants.O_NONBLOCK || 0) | (fs.constants.O_NOFOLLOW || 0));
  try {
    const stat = fs.fstatSync(fd);
    if (!stat.isFile() || stat.size < minimum || stat.size > maximum) {
      throw new Error('Invalid candidate input size');
    }
    // Read at most the inspected size plus one byte, including when the file grows.
    const bytes = Buffer.alloc(stat.size + 1);
    let length = 0;
    while (length < bytes.length) {
      const count = fs.readSync(fd, bytes, length, bytes.length - length, length);
      if (count === 0) break;
      length += count;
    }
    if (length !== stat.size) throw new Error('Candidate input changed while reading');
    return bytes.subarray(0, length);
  } finally {
    fs.closeSync(fd);
  }
}

module.exports = { readBytes };
