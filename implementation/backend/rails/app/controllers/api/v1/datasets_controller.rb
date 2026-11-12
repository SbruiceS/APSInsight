# frozen_string_literal: true

module Api
  module V1
    class DatasetsController < ApplicationController
      def index
        datasets = Dataset.includes(:project, :organization).order(created_at: :desc)
        render json: datasets.as_json(include: [:project, :organization])
      end

      def show
        dataset = Dataset.find(params[:id])
        render json: dataset.as_json(
          include: [:project, :organization, :dataset_versions],
          methods: [:current_version]
        )
      end

      def create
        dataset = Dataset.new(dataset_params)
        if dataset.save
          AuditEvent.create!(action: "create", resource_type: "Dataset", resource_id: dataset.id, metadata: { name: dataset.name })
          render json: dataset, status: :created
        else
          render json: { errors: dataset.errors.full_messages }, status: :unprocessable_entity
        end
      end

      def update
        dataset = Dataset.find(params[:id])
        if dataset.update(dataset_params)
          render json: dataset
        else
          render json: { errors: dataset.errors.full_messages }, status: :unprocessable_entity
        end
      end

      def schema
        dataset = Dataset.find(params[:id])
        version = dataset.current_version
        schema = dataset.current_schema
        render json: {
          version_snapshot: version&.schema_snapshot,
          current_schema: schema&.as_json(include: :dataset_columns)
        }
      end

      def preview
        dataset = Dataset.find(params[:id])
        # In real implementation this would stream or page from storage
        render json: {
          dataset_id: dataset.id,
          preview_rows: [],
          message: "Preview available after successful import and storage integration"
        }
      end

      private

      def dataset_params
        params.require(:dataset).permit(:project_id, :organization_id, :name, :slug, :description, :status, metadata: {})
      end
    end
  end
end
