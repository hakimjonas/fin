"use strict";
var __createBinding = (this && this.__createBinding) || (Object.create ? (function(o, m, k, k2) {
    if (k2 === undefined) k2 = k;
    var desc = Object.getOwnPropertyDescriptor(m, k);
    if (!desc || ("get" in desc ? !m.__esModule : desc.writable || desc.configurable)) {
      desc = { enumerable: true, get: function() { return m[k]; } };
    }
    Object.defineProperty(o, k2, desc);
}) : (function(o, m, k, k2) {
    if (k2 === undefined) k2 = k;
    o[k2] = m[k];
}));
var __setModuleDefault = (this && this.__setModuleDefault) || (Object.create ? (function(o, v) {
    Object.defineProperty(o, "default", { enumerable: true, value: v });
}) : function(o, v) {
    o["default"] = v;
});
var __importStar = (this && this.__importStar) || (function () {
    var ownKeys = function(o) {
        ownKeys = Object.getOwnPropertyNames || function (o) {
            var ar = [];
            for (var k in o) if (Object.prototype.hasOwnProperty.call(o, k)) ar[ar.length] = k;
            return ar;
        };
        return ownKeys(o);
    };
    return function (mod) {
        if (mod && mod.__esModule) return mod;
        var result = {};
        if (mod != null) for (var k = ownKeys(mod), i = 0; i < k.length; i++) if (k[i] !== "default") __createBinding(result, mod, k[i]);
        __setModuleDefault(result, mod);
        return result;
    };
})();
Object.defineProperty(exports, "__esModule", { value: true });
const core = __importStar(require("@actions/core"));
const child_process_1 = require("child_process");
const fs_1 = require("fs");
const path_1 = require("path");
const xml2js_1 = require("xml2js");
const function_1 = require("fp-ts/function");
const E = __importStar(require("fp-ts/Either"));
const TE = __importStar(require("fp-ts/TaskEither"));
const O = __importStar(require("fp-ts/Option"));
// Constants and types
const PACKAGE_NAME = 'fin';
const TARGET_DIR = 'target';
const ARTIFACT_DIR = (0, path_1.join)(TARGET_DIR, 'artifacts');
const DEB_ARCH = 'amd64';
const PLATFORMS = ['solus', 'arch', 'nix'];
// Ensure directories exist
const ensureDirectoryExists = (dirPath) => {
    if (!(0, fs_1.existsSync)(dirPath)) {
        (0, fs_1.mkdirSync)(dirPath, { recursive: true });
        core.info(`📁 Created directory: ${dirPath}`);
    }
};
ensureDirectoryExists(ARTIFACT_DIR);
// Helper: try to extract --tag argument from process.argv for local testing.
const getProvidedTag = () => {
    const fromCore = core.getInput('tag');
    if (fromCore && fromCore.trim().length > 0)
        return fromCore;
    const tagIndex = process.argv.findIndex(arg => arg === '--tag');
    if (tagIndex !== -1 && process.argv.length > tagIndex + 1) {
        return process.argv[tagIndex + 1];
    }
    return undefined;
};
// Helper functions
const executeCommand = (command) => TE.tryCatch(() => Promise.resolve((0, child_process_1.execSync)(command, { stdio: 'pipe' }).toString().trim()), (reason) => new Error(`Command failed: ${command}\n${String(reason)}`));
// Accept versions like "0.2.5", "v0.2.5", "0.2.5-suffix", or "v0.2.5-suffix"
const validateSemver = (version) => /^[0-9]+\.[0-9]+\.[0-9]+(?:-[\w.-]+)?(?:\+[\w.-]+)?$/.test(version)
    ? E.right(version)
    : E.left(new Error(`Invalid semantic version: '${version}'`));
// Parse only the core X.Y.Z parts for bumping (ignores suffixes)
const parseVersion = (version) => {
    const coreMatch = version.match(/^([0-9]+)\.([0-9]+)\.([0-9]+)/);
    if (coreMatch) {
        return E.right({
            major: parseInt(coreMatch[1], 10),
            minor: parseInt(coreMatch[2], 10),
            patch: parseInt(coreMatch[3], 10),
        });
    }
    return E.left(new Error(`Invalid version format: ${version}`));
};
// Tag determination logic
const getLatestGitTag = () => (0, function_1.pipe)(executeCommand('git describe --tags --abbrev=0'), TE.map(O.some), TE.orElse(() => TE.right(O.none)));
/**
 * If a tag is provided (nonempty), use it directly (after stripping a leading 'v' if present).
 * Otherwise, lookup the latest git tag and bump its patch version.
 */
const determineVersion = (providedTag) => {
    const cleanProvided = providedTag && providedTag.trim().length > 0
        ? providedTag.startsWith('v')
            ? providedTag.slice(1)
            : providedTag
        : undefined;
    if (cleanProvided) {
        // Validate and return the provided version as-is.
        return (0, function_1.pipe)(validateSemver(cleanProvided), E.map(() => cleanProvided), TE.fromEither);
    }
    else {
        // No tag provided: use latest git tag and bump its patch version.
        return (0, function_1.pipe)(getLatestGitTag(), TE.chain(O.fold(() => TE.left(new Error('No version tag found in Git history')), (tag) => TE.right(tag.startsWith('v') ? tag.slice(1) : tag))), TE.chain((version) => (0, function_1.pipe)(validateSemver(version), E.map(() => version), TE.fromEither)), TE.chain((version) => (0, function_1.pipe)(parseVersion(version), E.map((parts) => `${parts.major}.${parts.minor}.${parts.patch + 1}`), TE.fromEither)));
    }
};
// XML handling (asynchronous)
const updateFinSolVersion = (version) => (content) => (0, function_1.pipe)(TE.tryCatch(() => (0, xml2js_1.parseStringPromise)(content), (reason) => new Error(`XML parse error: ${String(reason)}`)), TE.chain((parsed) => {
    if (parsed?.Package?.Version?.[0]) {
        parsed.Package.Version[0] = version;
        return TE.right(new xml2js_1.Builder().buildObject(parsed));
    }
    return TE.left(new Error('Invalid fin.sol structure'));
}));
// File operations with asynchronous updater
const updateFile = (path, updater) => (0, function_1.pipe)(TE.tryCatch(() => Promise.resolve((0, fs_1.readFileSync)(path, 'utf8')), (reason) => new Error(`Read failed: ${path} - ${String(reason)}`)), TE.chain((content) => updater(content)), TE.chain((updated) => TE.tryCatch(() => {
    (0, fs_1.writeFileSync)(path, updated);
    return Promise.resolve();
}, (reason) => new Error(`Write failed: ${path} - ${String(reason)}`))));
// Version update for INSTALL.md
const updateInstallMd = (version) => (content) => {
    // Match any version pattern in the filename (including optional suffix)
    const versionPattern = '([0-9]+\\.[0-9]+\\.[0-9]+(?:-[\\w\\.-]+)?(?:\\+[\\w\\.-]+)?)';
    const patterns = PLATFORMS.map((p) => new RegExp(`${PACKAGE_NAME}-${versionPattern}-${p}\\.tar\\.gz`, 'g'));
    const debPattern = new RegExp(`${PACKAGE_NAME}_${versionPattern}_${DEB_ARCH}\\.deb`, 'g');
    let updated = content;
    let replacements = 0;
    // Update platform packages
    PLATFORMS.forEach((p, i) => {
        updated = updated.replace(patterns[i], (match) => {
            if (!match.includes(version)) {
                replacements++;
                return `${PACKAGE_NAME}-${version}-${p}.tar.gz`;
            }
            return match;
        });
    });
    // Update Debian package
    updated = updated.replace(debPattern, (match) => {
        const currentVersion = match.split('_')[1];
        if (currentVersion !== version) {
            replacements++;
            return `${PACKAGE_NAME}_${version}_${DEB_ARCH}.deb`;
        }
        return match;
    });
    return replacements > 0
        ? TE.right(updated)
        : TE.left(new Error('Version already up-to-date in INSTALL.md. No replacements needed.'));
};
// Update Cargo.toml: replace a line like version = "..."
const updateCargoToml = (version) => updateFile('Cargo.toml', (content) => {
    const newContent = content.replace(/version\s*=\s*"(.*?)"/, `version = "${version}"`);
    return newContent !== content
        ? TE.right(newContent)
        : TE.left(new Error('Version not updated in Cargo.toml'));
});
// Update CHANGELOG.md: Prepend a new changelog entry.
const updateChangelog = (version) => updateFile('CHANGELOG.md', (content) => {
    const today = new Date().toISOString().split('T')[0];
    const newEntry = `## [${version}] - ${today}\n\n- Description of changes.\n\n`;
    const newContent = newEntry + content;
    return TE.right(newContent);
});
// Update other platform files (e.g. PKGBUILD, flake.nix)
const updatePlatformFile = (path, pattern, replacement) => updateFile(path, (content) => {
    const updated = content.replace(pattern, replacement);
    return updated !== content
        ? TE.right(updated)
        : TE.left(new Error(`Pattern not found in ${path}`));
});
// Main workflow
const run = async () => {
    const providedTag = getProvidedTag();
    await (0, function_1.pipe)(
    // Use the provided tag (if any); otherwise, auto-detect and bump.
    determineVersion(providedTag), TE.chain((version) => {
        core.info(`🚀 Starting sync process for version: ${version}`);
        return TE.right(version);
    }), TE.chain((version) => (0, function_1.pipe)(updateFile('fin.sol', updateFinSolVersion(version)), TE.chain(() => updateFile('INSTALL.md', updateInstallMd(version))), TE.chain(() => updateCargoToml(version)), TE.chain(() => updateChangelog(version)), TE.chain(() => updatePlatformFile('PKGBUILD', /pkgver=\d+\.\d+\.\d+(?:-[\w.-]+)?(?:\+[\w.-]+)?/, `pkgver=${version}`)), TE.chain(() => updatePlatformFile('flake.nix', /version = "\d+\.\d+\.\d+(?:-[\w.-]+)?(?:\+[\w.-]+)?"/, `version = "${version}"`)), TE.map(() => version))), TE.match((error) => core.setFailed(error.message), () => core.info('✅ Sync process completed successfully')))();
};
run().catch((error) => core.setFailed(error instanceof Error ? error.message : 'Unknown error'));
