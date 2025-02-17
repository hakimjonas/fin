import * as core from '@actions/core';
import {execSync} from 'child_process';
import {existsSync, readFileSync, writeFileSync} from 'fs';
import {join} from 'path';

const PACKAGE_NAME = "fin";
const TARGET_DIR = "target";
const ARTIFACT_DIR = join(TARGET_DIR, "artifacts");
const DEB_ARCH = "amd64";
// Define the platforms for packaging
const PLATFORMS = ["solus", "arch", "nix"];

function determineVersion(providedTag: string | undefined): string {
    let TAG = providedTag || "";
    let tagVersion = "";
    // If no tag provided, try GITHUB_REF (for tag pushes)
    if (!TAG && process.env.GITHUB_REF) {
        TAG = process.env.GITHUB_REF.split('/').pop() || "";
        core.info(`🔍 Using tag from GITHUB_REF: ${TAG}`);
    }
    if (TAG) {
        tagVersion = TAG.startsWith("v") ? TAG.slice(1) : TAG;
        core.info(`ℹ️ Using provided version: ${tagVersion}`);
    } else if (existsSync(".git")) {
        try {
            tagVersion = execSync('git describe --tags --abbrev=0').toString().trim();
            tagVersion = tagVersion.startsWith("v") ? tagVersion.slice(1) : tagVersion;
        } catch (error) {
            core.warning("Could not determine version from git tags.");
        }
    }
    // Validate semantic version format
    const semverRegex = /^[0-9]+\.[0-9]+\.[0-9]+$/;
    if (!semverRegex.test(tagVersion)) {
        core.setFailed(`❌ Invalid/missing semantic version: '${tagVersion}'`);
        throw new Error(`Invalid/missing semantic version: '${tagVersion}'`);
    }
    // If no tag was provided originally, bump the patch version
    if (!providedTag) {
        const [major, minor, patch] = tagVersion.split('.').map(Number);
        tagVersion = `${major}.${minor}.${patch + 1}`;
        core.info(`🔼 Bumped version to: ${tagVersion}`);
    }
    return tagVersion;
}

function updateCargoToml(version: string) {
    try {
        execSync(`cargo set-version ${version}`, {stdio: 'inherit'});
        core.info(`📦 Updated Cargo.toml to version ${version}`);
    } catch (error) {
        core.setFailed("Failed to update Cargo.toml");
        throw error;
    }
}

function updateInstallMd(version: string) {
    try {
        // Update INSTALL.md for multiple patterns
        // This sed command uses extended regex and in-place editing.
        execSync(`sed -i -E \
      -e "s/(${PACKAGE_NAME}-)[0-9]+\\.[0-9]+\\.[0-9]+(-arch\\.tar\\.gz)/\\1${version}\\2/g" \
      -e "s/(${PACKAGE_NAME}-)[0-9]+\\.[0-9]+\\.[0-9]+(-nix\\.tar\\.gz)/\\1${version}\\2/g" \
      -e "s/(${PACKAGE_NAME}-)[0-9]+\\.[0-9]+\\.[0-9]+(-solus\\.tar\\.gz)/\\1${version}\\2/g" \
      -e "s/(${PACKAGE_NAME}_[0-9]+\\.[0-9]+\\.[0-9]+)(_${DEB_ARCH}\\.deb)/\\1${version}\\2/g" \
      INSTALL.md`);
        core.info("📄 Updated INSTALL.md");
    } catch (error) {
        core.setFailed("Failed to update INSTALL.md");
        throw error;
    }
}

function updateChangelog(version: string) {
    const changelogFile = "CHANGELOG.md";
    const releaseDate = execSync("date +%Y-%m-%d").toString().trim();
    core.info("📝 Generating changelog entry...");
    let lastTag = "";
    try {
        lastTag = execSync('git tag --sort=-v:refname')
            .toString()
            .split("\n")
            .find(tag => /^[0-9]+\.[0-9]+\.[0-9]+$/.test(tag.replace(/^v/, ""))) || "";
        lastTag = lastTag.replace(/^v/, "");
    } catch (error) {
        core.warning("Could not determine last tag.");
    }
    let newEntry = `## [${version}] - ${releaseDate}\n\n`;
    if (lastTag) {
        newEntry += `### Changes since ${lastTag}:\n`;
        try {
            const commitLog = execSync(`git log "v${lastTag}"..HEAD --pretty=format:"- %s (%h)"`).toString().trim();
            newEntry += commitLog ? `${commitLog}\n` : "No significant changes detected\n";
        } catch (error) {
            newEntry += "No significant changes detected\n";
        }
    } else {
        newEntry += "### Initial Release\n";
    }
    // Prepend the new entry to the changelog
    let existingChangelog = "";
    if (existsSync(changelogFile)) {
        existingChangelog = readFileSync(changelogFile, {encoding: "utf8"});
    }
    writeFileSync(changelogFile, newEntry + "\n" + existingChangelog, {encoding: "utf8"});
    core.info("CHANGELOG.md updated.");
}

async function run(): Promise<void> {
    try {
        const providedTag = core.getInput('tag');
        const version = determineVersion(providedTag);
        core.info(`Using version: ${version}`);
        updateCargoToml(version);
        updateInstallMd(version);
        updateChangelog(version);
        core.info('Sync process completed successfully!');
    } catch (error: any) {
        core.setFailed(error.message);
    }
}

run().then(r => r).catch(e => core.setFailed(e));
