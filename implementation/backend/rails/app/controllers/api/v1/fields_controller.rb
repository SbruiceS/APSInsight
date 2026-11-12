# frozen_string_literal: true

module Api
  module V1
    class FieldsController < ApplicationController
      def index
        fields = Field.includes(:farm).order(:name)
        render json: fields.as_json(include: :farm)
      end

      def show
        field = Field.find(params[:id])
        render json: field.as_json(include: [:farm, :plots])
      end

      def create
        field = Field.new(field_params)
        if field.save
          AuditEvent.create!(action: "create", resource_type: "Field", resource_id: field.id)
          render json: field, status: :created
        else
          render json: { errors: field.errors.full_messages }, status: :unprocessable_entity
        end
      end

      private

      def field_params
        params.require(:field).permit(:farm_id, :name, :area_hectares, boundary: {})
      end
    end
  end
end
