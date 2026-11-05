// Writes the cases of vectors.js to vectors/*.json:
// `node gnome-common/tests/write-vectors.js`.
import {mkdirSync, writeFileSync} from 'node:fs';

import {build} from './vectors.js';

const directory = new URL('./vectors/', import.meta.url);

mkdirSync(directory, {recursive: true});
for (const [name, cases] of Object.entries(build()))
    writeFileSync(new URL(`${name}.json`, directory), `${JSON.stringify(cases, null, 2)}\n`);
