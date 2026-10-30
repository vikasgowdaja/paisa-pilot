import assert from 'node:assert/strict'
import { mkdtemp, mkdir, readFile, rm, writeFile } from 'node:fs/promises'
import { tmpdir } from 'node:os'
import { join } from 'node:path'
import test from 'node:test'
import { prepareAndroid } from './prepare-android.mjs'

test('prepares Android app with native auth dependency idempotently', async () => {
  const project = await createAndroidFixture()
  try {
    await prepareAndroid(project)
    await prepareAndroid(project)
    const gradle = await readFile(join(project, 'app', 'build.gradle.kts'), 'utf8')
    const manifest = await readFile(join(project, 'app', 'src', 'main', 'AndroidManifest.xml'), 'utf8')
    const plugin = await readFile(
      join(project, 'app', 'src', 'main', 'java', 'com', 'paisapilot', 'gmail', 'GmailAuthPlugin.kt'),
      'utf8',
    )
    assert.equal(gradle.split('play-services-auth:21.4.0').length - 1, 1)
    assert.match(gradle, /ndkVersion = "28\.2\.13676358"/)
    assert.equal(manifest.split('android.permission.INTERNET').length - 1, 1)
    assert.match(plugin, /gmail\.readonly/)
    const activity = await readFile(
      join(project, 'app', 'src', 'main', 'java', 'com', 'paisapilot', 'desktop', 'MainActivity.kt'),
      'utf8',
    )
    assert.match(activity, /setOnApplyWindowInsetsListener/)
    const launcher = await readFile(
      join(project, 'app', 'src', 'main', 'res', 'mipmap-anydpi-v26', 'ic_launcher.xml'),
      'utf8',
    )
    assert.match(launcher, /@mipmap\/ic_launcher_foreground/)
  } finally {
    await rm(project, { recursive: true, force: true })
  }
})

test('adds release signing from environment without writing secrets to Gradle', async () => {
  const project = await createAndroidFixture()
  try {
    await prepareAndroid(project, { signing: true })
    const gradle = await readFile(join(project, 'app', 'build.gradle.kts'), 'utf8')
    assert.match(gradle, /PAISAPILOT_ANDROID_KEYSTORE_PASSWORD/)
    assert.match(gradle, /signingConfig = signingConfigs\.getByName\("release"\)/)
    assert.doesNotMatch(gradle, /keystore\.properties/)
  } finally {
    await rm(project, { recursive: true, force: true })
  }
})

test('pins NDK 28 when the generated Gradle file has no NDK version', async () => {
  const project = await createAndroidFixture()
  try {
    const gradlePath = join(project, 'app', 'build.gradle.kts')
    const gradle = (await readFile(gradlePath, 'utf8')).replace(
      '    ndkVersion = "27.0.12077973"\n',
      '',
    )
    await writeFile(gradlePath, gradle)
    await prepareAndroid(project)
    assert.match(await readFile(gradlePath, 'utf8'), /ndkVersion = "28\.2\.13676358"/)
  } finally {
    await rm(project, { recursive: true, force: true })
  }
})

test('throws a helpful error when Android scaffolding has not been initialized', async () => {
  const directory = await mkdtemp(join(tmpdir(), 'paisapilot-android-missing-'))
  try {
    await assert.rejects(
      () => prepareAndroid(directory),
      /Run "npm run android:init" and retry/,
    )
  } finally {
    await rm(directory, { recursive: true, force: true })
  }
})

async function createAndroidFixture() {
  const directory = await mkdtemp(join(tmpdir(), 'paisapilot-android-'))
  await mkdir(join(directory, 'app', 'src', 'main'), { recursive: true })
  await writeFile(
    join(directory, 'app', 'src', 'main', 'AndroidManifest.xml'),
    '<manifest xmlns:android="http://schemas.android.com/apk/res/android"><application /></manifest>\n',
  )
  await writeFile(
    join(directory, 'app', 'build.gradle.kts'),
    `android {
        ndkVersion = "27.0.12077973"
        buildTypes {
        getByName("release") {
            isMinifyEnabled = false
        }
    }
}

dependencies {
    implementation("androidx.core:core-ktx:1.15.0")
}
`,
  )
  return directory
}
