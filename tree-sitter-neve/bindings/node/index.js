const root = require('path');
const binding = require('node-gyp-build')(root.join(__dirname, '..', '..'));
module.exports = binding;
