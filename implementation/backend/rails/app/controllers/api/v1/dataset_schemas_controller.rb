# frozen_string_literal: true

module Api
  module V1
    class DatasetSchemasController < ApplicationController
      def index
        schemas = DatasetSchema.includes(:dataset, :dataset_columns).order(created_at: :desc)
        render json: schemas.as_json(include: [:dataset, :dataset_columns])
      end

      def show
        schema = DatasetSchema.find(params[:id])
        render json: schema.as_json(include: [:dataset, :dataset_columns])
      end

      def create
        schema = DatasetSchema.new(dataset_schema_params)
        if schema.save
          AuditEvent.create!(action: "create", resource_type: "DatasetSchema", resource_id: schema.id)
          render json: schema, status: :created
        else
          render json: { errors: schema.errors.full_messages }, status: :unprocessable_entity
        end
      end

      private

      def dataset_schema_params
        params.require(:dataset_schema).permit(:dataset_id, :version, raw_schema: {})
      end
    end
  end
end
