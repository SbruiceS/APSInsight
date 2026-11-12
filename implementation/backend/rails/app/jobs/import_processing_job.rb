# frozen_string_literal: true

class ImportProcessingJob < ApplicationJob
  queue_as :imports

  def perform(import_id)
    import = Import.find(import_id)
    import.update!(status: "processing")

    begin
      storage = ObjectStorage.new
      diagnostics = {
        processed_at: Time.current.iso8601,
        source_type: import.source_type,
        filename: import.original_filename,
        message: "Import processed"
      }

      # Create a data quality report stub tied to this import
      DataQualityReport.create!(
        dataset: import.dataset,
        import: import,
        status: "completed",
        summary: { source_type: import.source_type, note: "Profiler integration pending file content" },
        row_count: nil
      )

      import.update!(status: "succeeded", diagnostics: diagnostics)

      AuditEvent.create!(
        action: "import_succeeded",
        resource_type: "Import",
        resource_id: import.id,
        metadata: diagnostics
      )
    rescue StandardError => e
      import.update!(status: "failed", error_message: e.message)
      AuditEvent.create!(
        action: "import_failed",
        resource_type: "Import",
        resource_id: import.id,
        metadata: { error: e.message }
      )
      raise
    end
  end
end
