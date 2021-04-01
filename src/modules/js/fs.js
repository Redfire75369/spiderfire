export const readBinary = ______fsInternal______.readBinary;
export const readString = ______fsInternal______.readString;
export const readDir = ______fsInternal______.readDir;

export const write = ______fsInternal______.write;
export const createDir = ______fsInternal______.createDir;
export const createDirRecursive = ______fsInternal______.createDirRecursive;
export const removeFile = ______fsInternal______.removeFile;
export const removeDir = ______fsInternal______.removeDir;
export const removeDirRecursive = ______fsInternal______.removeDirRecursive;

export const copy = ______fsInternal______.copy;
export const rename = ______fsInternal______.rename;

export const softLink = ______fsInternal______.softLink;
export const hardLink = ______fsInternal______.hardLink;

const fs = Object.freeze({
	readBinary,
	readString,
	readDir,

	write,
	createDir,
	createDirRecursive,
	removeFile,
	removeDir,
	removeDirRecursive,

	copy,
	rename,

	softLink,
	hardLink
});

export default fs;
