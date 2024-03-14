import typescript from "@rollup/plugin-typescript";
import resolve from '@rollup/plugin-node-resolve';
import commonjs from '@rollup/plugin-commonjs';

export default {
    compilerOptions: {


    },
    plugins: [
        typescript(),
        resolve(),
        commonjs()
    ],
    input: 'frontend/src/main.ts',
    output: {
        file: 'dist/bundle.js',
    }
};