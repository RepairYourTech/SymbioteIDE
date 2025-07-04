/*---------------------------------------------------------------------------------------------
 *  Copyright (c) SymbioteIDE. All rights reserved.
 *  Licensed under the MIT License.
 *--------------------------------------------------------------------------------------------*/

'use strict';

const path = require('path');
const webpack = require('webpack');
const HtmlWebpackPlugin = require('html-webpack-plugin');
const MiniCssExtractPlugin = require('mini-css-extract-plugin');
const TerserPlugin = require('terser-webpack-plugin');
const CssMinimizerPlugin = require('css-minimizer-webpack-plugin');
const CopyWebpackPlugin = require('copy-webpack-plugin');

const mode = process.env.NODE_ENV || 'development';
const isProduction = mode === 'production';

module.exports = {
	mode: mode,
	target: 'web',
	entry: {
		main: './src/main.ts',
		workbench: './src/vs/workbench/workbench.main.ts',
		'editor.worker': 'monaco-editor/esm/vs/editor/editor.worker.js',
		'json.worker': 'monaco-editor/esm/vs/language/json/json.worker',
		'css.worker': 'monaco-editor/esm/vs/language/css/css.worker',
		'html.worker': 'monaco-editor/esm/vs/language/html/html.worker',
		'ts.worker': 'monaco-editor/esm/vs/language/typescript/ts.worker'
	},
	output: {
		path: path.resolve(__dirname, 'dist'),
		filename: isProduction ? '[name].[contenthash].js' : '[name].js',
		chunkFilename: isProduction ? '[name].[contenthash].chunk.js' : '[name].chunk.js',
		clean: true,
		globalObject: 'self'
	},
	resolve: {
		extensions: ['.ts', '.tsx', '.js', '.jsx', '.json'],
		alias: {
			'vs': path.resolve(__dirname, 'src/vs'),
			'@symbiote': path.resolve(__dirname, 'src'),
			'monaco-editor': path.resolve(__dirname, 'node_modules/monaco-editor')
		},
		fallback: {
			'fs': false,
			'path': require.resolve('path-browserify'),
			'crypto': require.resolve('crypto-browserify'),
			'stream': require.resolve('stream-browserify'),
			'buffer': require.resolve('buffer'),
			'util': require.resolve('util'),
			'assert': require.resolve('assert'),
			'process': require.resolve('process/browser'),
			'os': require.resolve('os-browserify/browser'),
			'url': require.resolve('url'),
			'http': false,
			'https': false,
			'child_process': false,
			'net': false,
			'tls': false,
			'dns': false
		}
	},
	module: {
		rules: [
			{
				test: /\.tsx?$/,
				use: 'ts-loader',
				exclude: /node_modules/
			},
			{
				test: /\.css$/,
				use: [
					isProduction ? MiniCssExtractPlugin.loader : 'style-loader',
					'css-loader'
				]
			},
			{
				test: /\.scss$/,
				use: [
					isProduction ? MiniCssExtractPlugin.loader : 'style-loader',
					'css-loader',
					'sass-loader'
				]
			},
			{
				test: /\.(woff|woff2|eot|ttf|otf)$/,
				type: 'asset/resource',
				generator: {
					filename: 'fonts/[name][ext]'
				}
			},
			{
				test: /\.(png|svg|jpg|jpeg|gif)$/,
				type: 'asset/resource',
				generator: {
					filename: 'images/[name][ext]'
				}
			},
			{
				test: /\.html$/,
				use: 'html-loader'
			}
		]
	},
	plugins: [
		new webpack.DefinePlugin({
			'process.env.NODE_ENV': JSON.stringify(mode),
			'process.env.BUILD_TIME': JSON.stringify(new Date().toISOString()),
			'process.env.VERSION': JSON.stringify(require('./package.json').version || '1.0.0')
		}),
		new webpack.ProvidePlugin({
			process: 'process/browser',
			Buffer: ['buffer', 'Buffer']
		}),
		new HtmlWebpackPlugin({
			template: './src/index.html',
			filename: 'index.html',
			chunks: ['main', 'workbench'],
			minify: isProduction ? {
				removeComments: true,
				collapseWhitespace: true,
				removeAttributeQuotes: true
			} : false
		}),
		new MiniCssExtractPlugin({
			filename: isProduction ? '[name].[contenthash].css' : '[name].css',
			chunkFilename: isProduction ? '[id].[contenthash].css' : '[id].css'
		}),
		new CopyWebpackPlugin({
			patterns: [
				{
					from: 'public',
					to: '.',
					globOptions: {
						ignore: ['**/index.html']
					}
				},
				{
					from: 'extensions',
					to: 'extensions'
				},
				{
					from: 'node_modules/monaco-editor/min/vs',
					to: 'vs'
				}
			]
		})
	],
	optimization: {
		minimize: isProduction,
		minimizer: [
			new TerserPlugin({
				terserOptions: {
					compress: {
						drop_console: isProduction,
						drop_debugger: isProduction
					},
					mangle: true,
					format: {
						comments: false
					}
				},
				extractComments: false
			}),
			new CssMinimizerPlugin()
		],
		splitChunks: {
			chunks: 'all',
			cacheGroups: {
				vendor: {
					test: /[\\/]node_modules[\\/]/,
					name: 'vendors',
					priority: 10,
					reuseExistingChunk: true
				},
				monaco: {
					test: /[\\/]node_modules[\\/]monaco-editor[\\/]/,
					name: 'monaco',
					priority: 20
				},
				common: {
					minChunks: 2,
					priority: 5,
					reuseExistingChunk: true
				}
			}
		},
		runtimeChunk: 'single'
	},
	devServer: {
		static: {
			directory: path.join(__dirname, 'public')
		},
		compress: true,
		port: 3000,
		hot: true,
		open: true,
		historyApiFallback: true,
		headers: {
			'Access-Control-Allow-Origin': '*',
			'Access-Control-Allow-Methods': 'GET, POST, PUT, DELETE, PATCH, OPTIONS',
			'Access-Control-Allow-Headers': 'X-Requested-With, content-type, Authorization'
		}
	},
	performance: {
		hints: isProduction ? 'warning' : false,
		maxEntrypointSize: 512000,
		maxAssetSize: 512000
	},
	stats: {
		colors: true,
		modules: false,
		children: false,
		chunks: false,
		chunkModules: false
	},
	devtool: isProduction ? 'source-map' : 'eval-source-map'
};