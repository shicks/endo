const tsNode = require('ts-node');
require('source-map-support').install();

tsNode.register({
  file: true,
  transpileOnly: true,
  project: 'tsconfig.json',
});
