# frozen_string_literal: true

module Api
  module V1
    class ImportsController < ApplicationController
      def create
        import = Import.new(import_params)
        import.status = "pending"
        if import.save
          ImportProcessingJob.perform_later(import.id)
          AuditEvent.create!(
            action: "import_queued",
            resource_type: "Import",
            resource_id: import.id,
            metadata: { dataset_id: import.dataset_id, source_type: import.source_type }
          )
          render json: import, status: :accepted
        else
          render json: { errors: import.errors.full_messages }, status: :unprocessable_entity
        end
      end

      def show
        import = Import.find(params[:id])
        render json: import
      end

      private

      def import_params
        params.require(:import).permit(:dataset_id, :project_id, :source_type, :original_filename)
      end
    end
  end
end