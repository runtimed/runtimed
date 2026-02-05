export function applyBufferPaths(
  data: Record<string, unknown>,
  bufferPaths: string[][] | undefined,
  buffers: ArrayBuffer[] | undefined
): Record<string, unknown> {
  if (!bufferPaths || !buffers || bufferPaths.length === 0) {
    return data;
  }

  for (let i = 0; i < bufferPaths.length && i < buffers.length; i++) {
    const path = bufferPaths[i];
    const buffer = buffers[i];

    if (path.length === 0) continue;

    // Navigate to the parent object
    let current: Record<string, unknown> = data;
    for (let j = 0; j < path.length - 1; j++) {
      const key = path[j];
      if (current[key] === undefined || current[key] === null) {
        current[key] = {};
      }
      current = current[key] as Record<string, unknown>;
    }

    // Set the buffer at the final key
    const finalKey = path[path.length - 1];
    current[finalKey] = buffer;
  }

  return data;
}

/**
 * Extract buffers from message data at specified paths.
 *
 * This is the reverse of applyBufferPaths - used when sending
 * messages from the frontend to the kernel.
 *
 * @param data - The message data object (mutated in place, buffer values set to null)
 * @param bufferPaths - Array of paths where buffers should be extracted from
 * @returns Array of extracted buffers (or undefined for paths that don't exist)
 */
export function extractBuffers(
  data: Record<string, unknown>,
  bufferPaths: string[][] | undefined
): ArrayBuffer[] {
  if (!bufferPaths || bufferPaths.length === 0) {
    return [];
  }

  const buffers: ArrayBuffer[] = [];

  for (const path of bufferPaths) {
    if (path.length === 0) {
      buffers.push(new ArrayBuffer(0));
      continue;
    }

    // Navigate to the parent object
    let current: Record<string, unknown> = data;
    let found = true;

    for (let j = 0; j < path.length - 1; j++) {
      const key = path[j];
      if (
        current[key] === undefined ||
        current[key] === null ||
        typeof current[key] !== "object"
      ) {
        found = false;
        break;
      }
      current = current[key] as Record<string, unknown>;
    }

    if (found) {
      const finalKey = path[path.length - 1];
      const value = current[finalKey];

      if (value instanceof ArrayBuffer) {
        buffers.push(value);
        // Replace with null in the data (per protocol spec)
        current[finalKey] = null;
      } else if (ArrayBuffer.isView(value) && value.buffer instanceof ArrayBuffer) {
        // Handle typed arrays (Uint8Array, etc.) with ArrayBuffer backing
        buffers.push(value.buffer.slice(value.byteOffset, value.byteOffset + value.byteLength));
        current[finalKey] = null;
      } else {
        // Path exists but value is not a buffer
        buffers.push(new ArrayBuffer(0));
      }
    } else {
      // Path not found
      buffers.push(new ArrayBuffer(0));
    }
  }

  return buffers;
}

/**
 * Find all ArrayBuffer values in an object and return their paths.
 *
 * Useful for automatically detecting buffer paths when sending data.
 *
 * @param data - The data object to scan
 * @param prefix - Current path prefix (for recursion)
 * @returns Array of paths to ArrayBuffer values
 */
export function findBufferPaths(
  data: Record<string, unknown>,
  prefix: string[] = []
): string[][] {
  const paths: string[][] = [];

  for (const [key, value] of Object.entries(data)) {
    const currentPath = [...prefix, key];

    if (value instanceof ArrayBuffer || ArrayBuffer.isView(value)) {
      paths.push(currentPath);
    } else if (value !== null && typeof value === "object" && !Array.isArray(value)) {
      // Recurse into nested objects
      paths.push(...findBufferPaths(value as Record<string, unknown>, currentPath));
    }
  }

  return paths;
}
