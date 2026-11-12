# frozen_string_literal: true

class ObjectStorage
  # Abstraction over local filesystem or S3-compatible storage.
  # Configure via ENV: STORAGE_BACKEND=local|s3, STORAGE_ROOT, S3_ENDPOINT, etc.

  def initialize
    @backend = ENV.fetch("STORAGE_BACKEND", "local")
    @root = ENV.fetch("STORAGE_ROOT", Rails.root.join("storage").to_s)
  end

  def put(key, content)
    case @backend
    when "local"
      path = File.join(@root, key)
      FileUtils.mkdir_p(File.dirname(path))
      File.binwrite(path, content)
      key
    else
      raise "S3 backend not yet implemented — set STORAGE_BACKEND=local"
    end
  end

  def get(key)
    case @backend
    when "local"
      path = File.join(@root, key)
      raise "Object not found: #{key}" unless File.exist?(path)
      File.binread(path)
    else
      raise "S3 backend not yet implemented"
    end
  end

  def exists?(key)
    case @backend
    when "local"
      File.exist?(File.join(@root, key))
    else
      false
    end
  end

  def url_for(key)
    "/storage/#{key}" # Placeholder; real S3 would return signed URL
  end
end
