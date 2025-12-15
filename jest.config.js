module.exports = {
    testEnvironment: 'jsdom',
    transform: {
        '^.+\\.[t|j]sx?$': 'babel-jest',
    },
    moduleNameMapper: {
        '\\.(css|less|scss|sass)$': 'identity-obj-proxy',
    },
    // Handle ESM modules that might be inside node_modules (if any in future)
    transformIgnorePatterns: ['/node_modules/'],
    testMatch: ['**/tests/**/*.test.js'],
};
