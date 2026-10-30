import { copyFile, cp, mkdir, readFile, writeFile } from 'node:fs/promises'
import { dirname, join, resolve } from 'node:path'
import { fileURLToPath } from 'node:url'

const repositoryRoot = resolve(dirname(fileURLToPath(import.meta.url)), '..')

export async function prepareAndroid(projectDirectory, { signing = false } = {}) {
  const appDirectory = join(projectDirectory, 'app')
  const gradleFile = join(appDirectory, 'build.gradle.kts')
  const manifestFile = join(appDirectory, 'src', 'main', 'AndroidManifest.xml')
  const pluginSource = join(repositoryRoot, 'src-tauri', 'android', 'GmailAuthPlugin.kt')
  const pluginDestination = join(
    appDirectory,
    'src',
    'main',
    'java',
    'com',
    'paisapilot',
    'gmail',
    'GmailAuthPlugin.kt',
  )

  const activitySource = join(repositoryRoot, 'src-tauri', 'android', 'MainActivity.kt')
  const activityDestination = join(
    appDirectory,
    'src',
    'main',
    'java',
    'com',
    'paisapilot',
    'desktop',
    'MainActivity.kt',
  )

  await mkdir(dirname(pluginDestination), { recursive: true })
  await copyFile(pluginSource, pluginDestination)
  await mkdir(dirname(activityDestination), { recursive: true })
  await copyFile(activitySource, activityDestination)

  // `tauri android init` generates Tauri's default launcher icons; replace them with ours.
  await cp(join(repositoryRoot, 'src-tauri', 'icons', 'android'), join(appDirectory, 'src', 'main', 'res'), {
    recursive: true,
    force: true,
  })

  let manifest = await readRequiredFile(manifestFile)
  const internetPermission = '<uses-permission android:name="android.permission.INTERNET" />'
  if (!manifest.includes(internetPermission)) {
    const applicationTag = /(<application\b)/
    if (!applicationTag.test(manifest)) {
      throw new Error(`Could not find application tag in ${manifestFile}`)
    }
    manifest = manifest.replace(applicationTag, `${internetPermission}\n    $1`)
    await writeFile(manifestFile, manifest)
  }

  let gradle = await readRequiredFile(gradleFile)
  const dependency = 'implementation("com.google.android.gms:play-services-auth:21.4.0")'
  if (!gradle.includes(dependency)) {
    const dependenciesBlock = /(^[ \t]*dependencies[ \t]*\{)/m
    if (!dependenciesBlock.test(gradle)) {
      throw new Error(`Could not find dependencies block in ${gradleFile}`)
    }
    gradle = gradle.replace(dependenciesBlock, `$1\n    ${dependency}`)
  }
  gradle = configureNdkVersion(gradle, gradleFile)
  gradle = configureArchiveName(gradle, gradleFile)

  if (signing) {
    gradle = configureReleaseSigning(gradle, gradleFile)
  }
  await writeFile(gradleFile, gradle)
}

async function readRequiredFile(filePath) {
  try {
    return await readFile(filePath, 'utf8')
  } catch (error) {
    if (error && error.code === 'ENOENT') {
      throw new Error(
        `Android project files are missing at ${filePath}. Run "npm run android:init" and retry.`,
        { cause: error },
      )
    }
    throw error
  }
}

function configureArchiveName(gradle, gradleFile) {
  const setting = 'base.archivesName.set("PaisaPilot")'
  if (gradle.includes(setting)) {
    return gradle
  }
  const androidBlock = /^[ \t]*android[ \t]*\{/m
  if (!androidBlock.test(gradle)) {
    throw new Error(`Could not find android block in ${gradleFile}`)
  }
  return gradle.replace(androidBlock, (match) => `${setting}\n\n${match}`)
}

function configureNdkVersion(gradle, gradleFile) {
  const ndkSetting = /(^[ \t]*ndkVersion[ \t]*=[ \t]*)"[^"]+"/m
  if (ndkSetting.test(gradle)) {
    return gradle.replace(ndkSetting, '$1"28.2.13676358"')
  }
  const androidBlock = /(^[ \t]*)android[ \t]*\{/m
  const match = gradle.match(androidBlock)
  if (!match) {
    throw new Error(`Could not find android block in ${gradleFile}`)
  }
  return gradle.replace(match[0], `${match[0]}\n${match[1]}    ndkVersion = "28.2.13676358"`)
}

function configureReleaseSigning(gradle, gradleFile) {
  if (!gradle.includes('create("release")')) {
    const buildTypesBlock = /(^[ \t]*)buildTypes[ \t]*\{/m
    const match = gradle.match(buildTypesBlock)
    if (!match) {
      throw new Error(`Could not find buildTypes block in ${gradleFile}`)
    }
    const indent = match[1]
    const signingConfig = `${indent}signingConfigs {
${indent}    create("release") {
${indent}        val keystorePath = System.getenv("PAISAPILOT_ANDROID_KEYSTORE")
${indent}            ?: error("PAISAPILOT_ANDROID_KEYSTORE is required for release signing")
${indent}        val keystorePassword = System.getenv("PAISAPILOT_ANDROID_KEYSTORE_PASSWORD")
${indent}            ?: error("PAISAPILOT_ANDROID_KEYSTORE_PASSWORD is required for release signing")
${indent}        val keyAliasValue = System.getenv("PAISAPILOT_ANDROID_KEY_ALIAS")
${indent}            ?: error("PAISAPILOT_ANDROID_KEY_ALIAS is required for release signing")
${indent}        storeFile = file(keystorePath)
${indent}        storePassword = keystorePassword
${indent}        keyAlias = keyAliasValue
${indent}        keyPassword = keystorePassword
${indent}    }
${indent}}

`
    gradle = gradle.replace(match[0], `${signingConfig}${match[0]}`)
  }

  const releaseBuildType = /(getByName\("release"\)[ \t]*\{)/
  if (!releaseBuildType.test(gradle)) {
    throw new Error(`Could not find release build type in ${gradleFile}`)
  }
  if (!gradle.includes('signingConfig = signingConfigs.getByName("release")')) {
    gradle = gradle.replace(
      releaseBuildType,
      '$1\n            signingConfig = signingConfigs.getByName("release")',
    )
  }
  return gradle
}

if (process.argv[1] && resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  const projectDirectory = join(repositoryRoot, 'src-tauri', 'gen', 'android')
  const signing = process.argv.includes('--signing')
  await prepareAndroid(projectDirectory, { signing })
}
